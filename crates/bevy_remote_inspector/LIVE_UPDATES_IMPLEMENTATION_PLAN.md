# Live Updates Implementation Plan for bevy_remote_inspector

## Overview

This document outlines the complete implementation plan for adding real-time component value streaming to the bevy_remote_inspector using Server-Sent Events (SSE) and the existing `bevy/get+watch` endpoint.

## Current Status

✅ **Working**: Static component viewing, virtual scrolling, entity selection  
❌ **Missing**: Live streaming updates (currently simulated with polling every 3 seconds)  

## Architecture Overview

```
Target App (bevy_remote) <--SSE--> Inspector HTTP Client <--> Inspector UI
    │                                       │                        │
    ├─ Transform changes                    ├─ SSE Connection         ├─ ComponentViewer
    ├─ Health/Speed updates                 ├─ Stream Parser          ├─ Auto-refresh
    └─ bevy/get+watch endpoint              └─ Update Queue           └─ Diff highlighting
```

## Implementation Plan

### Phase 1: Replace Simulation with Real SSE Client

**File**: `src/http_client.rs`

**Current Problem**: 
```rust
// CURRENT: Fake polling simulation
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(3));
    loop {
        interval.tick().await;
        // Simulate updates...
    }
});
```

**Solution**: Replace with real SSE connection
```rust
pub async fn start_watching_entity(&mut self, entity_id: u32, components: Vec<String>) -> Result<()> {
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: self.next_id(),
        method: "bevy/get+watch".to_string(),
        params: Some(serde_json::json!({
            "entity": entity_id,
            "components": components // Empty array = all components
        })),
    };

    // Establish SSE connection
    let url = format!("{}/jsonrpc", self.base_url);
    let response = self.client
        .post(&url)
        .header("Accept", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow!("Failed to start SSE stream: {}", response.status()));
    }

    let (tx, rx) = mpsc::unbounded_channel();
    self.update_receiver = Some(rx);
    
    // Stream parser task
    let entity_id_for_task = entity_id;
    tokio::spawn(async move {
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    buffer.push_str(&String::from_utf8_lossy(&chunk));
                    
                    // Process complete SSE messages
                    while let Some(end) = buffer.find("\n\n") {
                        let message = buffer[..end].to_string();
                        buffer = buffer[end + 2..].to_string();
                        
                        if let Some(json_data) = parse_sse_message(&message) {
                            if let Ok(response): Result<JsonRpcResponse, _> = serde_json::from_str(&json_data) {
                                if let Some(result) = response.result {
                                    let update = RemoteUpdate {
                                        entity_id: entity_id_for_task,
                                        components: parse_component_data(result),
                                        timestamp: std::time::SystemTime::now(),
                                    };
                                    
                                    if tx.send(update).is_err() {
                                        break; // Receiver dropped, stop streaming
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("SSE stream error: {}", e);
                    break;
                }
            }
        }
        println!("SSE stream ended for entity {}", entity_id_for_task);
    });

    Ok(())
}

fn parse_sse_message(message: &str) -> Option<String> {
    for line in message.lines() {
        if line.starts_with("data: ") {
            return Some(line[6..].to_string());
        }
    }
    None
}

fn parse_component_data(result: serde_json::Value) -> HashMap<String, serde_json::Value> {
    if let Some(components) = result.get("components") {
        if let Ok(component_map) = serde_json::from_value(components.clone()) {
            return component_map;
        }
    }
    HashMap::new()
}
```

**Key Changes**:
- Replace polling with real SSE connection using `text/event-stream`
- Parse SSE format: `data: {json}\n\n`
- Handle streaming errors and reconnection
- Add proper connection lifecycle management

### Phase 2: Update RemoteUpdate Structure

**File**: `src/http_client.rs`

**Enhanced Update Structure**:
```rust
#[derive(Debug, Clone)]
pub struct RemoteUpdate {
    pub entity_id: u32,
    pub components: HashMap<String, serde_json::Value>,
    pub removed_components: Vec<String>, // Handle component removal
    pub timestamp: std::time::SystemTime,
    pub update_type: UpdateType,
}

#[derive(Debug, Clone)]
pub enum UpdateType {
    ComponentChanged,
    ComponentAdded,
    ComponentRemoved,
    EntityRemoved,
}
```

### Phase 3: Connection Management

**File**: `src/http_client.rs`

**Stream Management**:
```rust
pub struct HttpRemoteClient {
    // ... existing fields
    pub active_streams: HashMap<u32, StreamHandle>, // Track per-entity streams
    pub stream_tasks: Vec<tokio::task::JoinHandle<()>>, // Clean up tasks
}

pub struct StreamHandle {
    pub entity_id: u32,
    pub components: Vec<String>,
    pub cancel_token: tokio_util::sync::CancellationToken,
}

impl HttpRemoteClient {
    pub async fn stop_watching_entity(&mut self, entity_id: u32) -> Result<()> {
        if let Some(handle) = self.active_streams.remove(&entity_id) {
            handle.cancel_token.cancel();
            println!("Stopped watching entity {}", entity_id);
        }
        Ok(())
    }
    
    pub async fn stop_all_streams(&mut self) {
        for (_, handle) in self.active_streams.drain() {
            handle.cancel_token.cancel();
        }
        
        // Wait for tasks to complete
        for task in self.stream_tasks.drain(..) {
            let _ = task.await;
        }
        
        println!("Stopped all SSE streams");
    }
}
```

### Phase 4: UI Integration - Real-time Component Updates

**File**: `src/ui/component_viewer.rs`

**Auto-refreshing Component Display**:
```rust
/// System to update component viewer with live data
pub fn update_component_viewer_live(
    mut http_client: ResMut<HttpRemoteClient>,
    selected_entity: Res<SelectedEntity>,
    mut text_query: Query<&mut Text, With<ComponentValueText>>,
    mut component_sections: Query<(Entity, &ComponentSection, &Children), With<CollapsibleContent>>,
    mut commands: Commands,
) {
    // Check for live updates
    let updates = http_client.check_updates();
    
    for update in updates {
        // Only update if this is the currently selected entity
        if let Some(selected_id) = selected_entity.entity_id {
            if update.entity_id == selected_id {
                update_component_display(&mut commands, &mut text_query, &mut component_sections, &update);
            }
        }
    }
}

fn update_component_display(
    commands: &mut Commands,
    text_query: &mut Query<&mut Text, With<ComponentValueText>>,
    component_sections: &mut Query<(Entity, &ComponentSection, &Children), With<CollapsibleContent>>,
    update: &RemoteUpdate,
) {
    for (section_entity, component_section, children) in component_sections.iter_mut() {
        if let Some(new_value) = update.components.get(&component_section.component_name) {
            // Find the text entity for this component
            for &child in children.iter() {
                if let Ok(mut text) = text_query.get_mut(child) {
                    let formatted_value = format_component_value(new_value);
                    
                    // Highlight if value changed
                    if text.0 != formatted_value {
                        text.0 = formatted_value;
                        
                        // Add visual indication of change (flash green)
                        commands.entity(child).insert(ValueChangedIndicator {
                            timer: Timer::from_seconds(0.5, TimerMode::Once),
                        });
                    }
                }
            }
        }
    }
}

/// Component to indicate a value just changed
#[derive(Component)]
struct ValueChangedIndicator {
    timer: Timer,
}

/// System to fade out change indicators
fn update_change_indicators(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut ValueChangedIndicator, &mut TextColor)>,
) {
    for (entity, mut indicator, mut color) in query.iter_mut() {
        indicator.timer.tick(time.delta());
        
        if indicator.timer.finished() {
            // Remove indicator and reset color
            commands.entity(entity).remove::<ValueChangedIndicator>();
            color.0 = Color::WHITE;
        } else {
            // Interpolate from green to white
            let t = indicator.timer.fraction();
            color.0 = Color::srgb(
                0.2 + 0.8 * t,  // Red: 0.2 -> 1.0
                1.0,            // Green: stays 1.0
                0.2 + 0.8 * t,  // Blue: 0.2 -> 1.0
            );
        }
    }
}
```

### Phase 5: Integration with Entity Selection

**File**: `src/ui/entity_list.rs`

**Auto-start/stop streaming based on selection**:
```rust
/// Enhanced entity selection that manages streaming
pub fn handle_entity_selection_with_streaming(
    mut selected_entity: ResMut<SelectedEntity>,
    interaction_query: Query<(&Interaction, &EntityListItem), Changed<Interaction>>,
    mut http_client: ResMut<HttpRemoteClient>,
    rt: Res<TokioRuntime>, // Add tokio runtime resource
) {
    for (interaction, item) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            let old_selection = selected_entity.entity_id;
            selected_entity.entity_id = Some(item.entity_id);
            
            // Stop watching old entity
            if let Some(old_id) = old_selection {
                if old_id != item.entity_id {
                    rt.spawn(async move {
                        let _ = http_client.stop_watching_entity(old_id).await;
                    });
                }
            }
            
            // Start watching new entity
            rt.spawn(async move {
                let _ = http_client.start_watching_entity(item.entity_id, vec![]).await;
            });
            
            println!("Started streaming updates for entity: {}", item.entity_id);
        }
    }
}
```

### Phase 6: Error Handling and Reconnection

**File**: `src/http_client.rs`

**Robust Error Handling**:
```rust
pub struct StreamConnectionState {
    pub is_connected: bool,
    pub last_error: Option<String>,
    pub reconnect_attempts: u32,
    pub max_reconnect_attempts: u32,
    pub reconnect_delay: std::time::Duration,
}

impl HttpRemoteClient {
    async fn reconnect_stream(&mut self, entity_id: u32, components: Vec<String>) -> Result<()> {
        let mut state = StreamConnectionState {
            is_connected: false,
            last_error: None,
            reconnect_attempts: 0,
            max_reconnect_attempts: 5,
            reconnect_delay: std::time::Duration::from_secs(1),
        };
        
        while !state.is_connected && state.reconnect_attempts < state.max_reconnect_attempts {
            match self.start_watching_entity(entity_id, components.clone()).await {
                Ok(_) => {
                    state.is_connected = true;
                    state.last_error = None;
                    println!("Reconnected to stream for entity {}", entity_id);
                }
                Err(e) => {
                    state.reconnect_attempts += 1;
                    state.last_error = Some(e.to_string());
                    
                    if state.reconnect_attempts < state.max_reconnect_attempts {
                        println!("Reconnection attempt {} failed for entity {}, retrying in {:?}", 
                            state.reconnect_attempts, entity_id, state.reconnect_delay);
                        tokio::time::sleep(state.reconnect_delay).await;
                        state.reconnect_delay *= 2; // Exponential backoff
                    }
                }
            }
        }
        
        if !state.is_connected {
            return Err(anyhow!("Failed to reconnect after {} attempts", state.max_reconnect_attempts));
        }
        
        Ok(())
    }
}
```

## Testing Strategy

### Test Cases

1. **Basic Streaming**: Watch a single entity with frequently changing Transform
2. **Component Addition/Removal**: Dynamically add/remove components and verify updates
3. **Multiple Entity Switching**: Rapidly switch between entities, verify streams start/stop correctly
4. **Connection Resilience**: Kill target app, restart, verify reconnection
5. **Performance**: Stream 100+ entities simultaneously (stress test)

### Test with Moving Target App

The existing `examples/moving_target_app.rs` is perfect for testing:
- Player with rapidly changing health, speed, energy, experience
- Enemies with AI state changes
- Fast-moving projectiles with decreasing lifetime
- Auto-spawning entities every 5 seconds

### Performance Considerations

**Client-side**:
- Limit concurrent streams (max 10-20 entities)
- Implement update throttling (max 60fps UI updates)
- Queue updates and batch UI changes

**Network**:
- SSE is efficient for real-time updates
- Only changed components are sent
- Consider compression for large component data

## Migration to bevy_dev_tools

When migrating this to `bevy_dev_tools`, the implementation should be structured as:

```
bevy_dev_tools/
├── src/
│   └── remote_inspector/
│       ├── mod.rs                    # Plugin definition
│       ├── http_client.rs           # SSE streaming client
│       ├── ui/
│       │   ├── mod.rs
│       │   ├── entity_list.rs       # Virtual scrolling list
│       │   ├── component_viewer.rs  # Live-updating viewer
│       │   └── connection_status.rs # Streaming status
│       └── streaming/
│           ├── mod.rs
│           ├── sse_client.rs        # SSE implementation
│           └── update_manager.rs    # Update queue/batching
```

## Dependencies Required

Add to `Cargo.toml`:
```toml
# For SSE streaming
tokio = { version = "1.0", features = ["rt-multi-thread", "time", "macros"] }
tokio-stream = "0.1"
tokio-util = "0.7"
futures-util = "0.3"

# For HTTP streaming
reqwest = { version = "0.12", features = ["json", "stream"] }
```

## Summary

This implementation plan provides:
1. **Real SSE streaming** replacing current simulation
2. **Robust connection management** with reconnection logic
3. **Live UI updates** with visual change indicators
4. **Entity selection integration** with automatic stream management
5. **Comprehensive error handling** and performance optimization
6. **Clear migration path** to bevy_dev_tools

The result will be a production-ready remote inspector with true real-time component value streaming, perfect for debugging dynamic Bevy applications.