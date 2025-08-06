# Live Component Updates Implementation Plan

## Overview

This plan implements live ticking component value updates for the bevy_dev_tools inspector using Bevy's built-in systems and capabilities. The implementation leverages bevy_remote's existing `bevy/get+watch` streaming functionality and bevy_tasks for async processing.

## Current Status

✅ **Working**: Static component viewing, virtual scrolling, entity selection  
✅ **Available**: bevy_remote's built-in `bevy/get+watch` with change detection
❌ **Missing**: Live streaming client integration and UI updates

## Architecture Overview

```
Target App (bevy_remote) <--Streaming--> Inspector HTTP Client <--> Inspector UI
    │                                           │                         │
    ├─ Change Detection                         ├─ bevy_tasks Async       ├─ ComponentViewer
    ├─ Component Ticks                          ├─ Streaming Client       ├─ Live Updates
    ├─ Removal Events                           ├─ Change Processing      ├─ Visual Indicators
    └─ bevy/get+watch (built-in)               └─ Update Queue           └─ Diff highlighting
```

## Detailed Implementation Plan

### Phase 1: HTTP Streaming Client (Using bevy_tasks)

#### 1.1 Enhanced HTTP Client with Watching Support
**File:** `crates/bevy_dev_tools/src/inspector/http_client.rs`

**Key insight:** bevy_remote's `bevy/get+watch` already provides:
- Change detection via `entity_ref.get_change_ticks_by_id(component_id)`
- Removed component tracking via `world.removed_components()` event cursors  
- Returns `None` when no changes, actual data when changed
- Built-in watching system that we can leverage

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

**Solution**: Use bevy_tasks with real `bevy/get+watch` streaming
```rust
use bevy_tasks::{AsyncComputeTaskPool, Task};
use async_channel::{Receiver, Sender};

pub struct HttpRemoteClient {
    // ... existing fields ...
    
    // New fields for streaming
    pub watching_tasks: HashMap<u32, Task<()>>,
    pub update_sender: Option<Sender<ComponentUpdate>>,
    pub update_receiver: Option<Receiver<ComponentUpdate>>,
    pub watched_entities: HashMap<u32, Vec<String>>, // entity -> components being watched
}

#[derive(Debug, Clone)]
pub struct ComponentUpdate {
    pub entity_id: u32,
    pub changed_components: HashMap<String, Value>,
    pub removed_components: Vec<String>,
    pub timestamp: f64,
}

impl HttpRemoteClient {
    /// Start watching components for an entity using bevy/get+watch
    pub fn start_component_watching(&mut self, entity_id: u32, components: Vec<String>) -> Result<()> {
        let (tx, rx) = async_channel::unbounded();
        self.update_sender = Some(tx.clone());
        self.update_receiver = Some(rx);
        
        let base_url = self.base_url.clone();
        let client = self.client.clone();
        
        // Spawn async task using bevy_tasks (no external tokio dependency needed)
        let task = AsyncComputeTaskPool::get().spawn(async move {
            // Use bevy/get+watch with continuous polling
            let request = BrpRequest {
                jsonrpc: "2.0".to_string(),
                method: "bevy/get+watch".to_string(),
                id: Some(serde_json::json!(entity_id)),
                params: Some(serde_json::json!({
                    "entity": entity_id,
                    "components": components,
                    "strict": false
                })),
            };
            
            // Long-lived connection for streaming (bevy_remote handles the watching internally)
            loop {
                match send_watch_request(&client, &base_url, &request).await {
                    Ok(Some(response)) => {
                        // bevy_remote only sends when components actually changed!
                        let update = ComponentUpdate {
                            entity_id,
                            changed_components: response.components,
                            removed_components: response.removed,
                            timestamp: current_time(),
                        };
                        
                        if tx.send(update).await.is_err() {
                            break; // Receiver dropped
                        }
                    },
                    Ok(None) => {
                        // No changes detected by bevy_remote's change detection system
                        // This is the key insight - bevy_remote handles the diffing for us!
                        std::thread::sleep(std::time::Duration::from_millis(16)); // ~60 FPS polling
                    },
                    Err(e) => {
                        println!("Watch connection error: {}", e);
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        // Retry connection
                    }
                }
            }
        });
        
        self.watching_tasks.insert(entity_id, task);
        self.watched_entities.insert(entity_id, components);
        Ok(())
    }
}

async fn send_watch_request(
    client: &reqwest::Client,
    base_url: &str,
    request: &BrpRequest,
) -> Result<Option<BrpGetWatchingResponse>> {
    let url = format!("{}/jsonrpc", base_url);
    let response = client
        .post(&url)
        .json(request)
        .send()
        .await?;

    let json_response: BrpResponse = response.json().await?;
    
    match json_response.payload {
        BrpPayload::Result(value) => {
            if value.is_null() {
                Ok(None) // No changes
            } else {
                let watch_response: BrpGetWatchingResponse = serde_json::from_value(value)?;
                Ok(Some(watch_response))
            }
        },
        BrpPayload::Error(err) => Err(anyhow!("Watch request error: {}", err.message)),
    }
}
```

### Phase 2: Component Value Diffing and Change Detection

#### 2.1 Change Detection System
**Key insight:** bevy_remote's `bevy/get+watch` already handles change detection using:
- `entity_ref.get_change_ticks_by_id(component_id)` for component changes
- `world.removed_components()` event cursors for removals
- Returns `None` when no changes, actual data when changed

#### 2.2 Component Cache with Change Tracking
**File:** `crates/bevy_dev_tools/src/inspector/ui/component_viewer.rs`

```rust
#[derive(Resource)]
pub struct LiveComponentCache {
    pub entity_components: HashMap<u32, HashMap<String, ComponentState>>,
    pub last_update_time: f64,
    pub update_frequency: f64, // Target update rate (e.g., 30 FPS)
}

#[derive(Debug, Clone)]
pub struct ComponentState {
    pub current_value: Value,
    pub last_changed_time: f64,
    pub change_indicator: ChangeIndicator,
    pub previous_value: Option<Value>, // For showing diffs
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChangeIndicator {
    Unchanged,
    Changed { duration: f64 }, // How long to show changed indicator
    Removed,
    Added,
}
```

### Phase 3: UI Update System for Live Values

#### 3.1 Live Component Display System
**File:** `crates/bevy_dev_tools/src/inspector/ui/live_components.rs`

```rust
/// System to process live component updates from HTTP client
pub fn process_live_component_updates(
    mut live_cache: ResMut<LiveComponentCache>,
    mut http_client: ResMut<HttpRemoteClient>,
    time: Res<Time>,
    selected_entity: Res<SelectedEntity>,
) {
    let current_time = time.elapsed_secs_f64();
    
    // Rate limiting - only process updates at target frequency
    if current_time - live_cache.last_update_time < 1.0 / live_cache.update_frequency {
        return;
    }
    
    // Process all pending updates
    if let Some(ref mut receiver) = http_client.update_receiver {
        while let Ok(update) = receiver.try_recv() {
            process_component_update(&mut live_cache, update, current_time);
        }
    }
    
    live_cache.last_update_time = current_time;
}

fn process_component_update(
    cache: &mut LiveComponentCache, 
    update: ComponentUpdate, 
    current_time: f64
) {
    let entity_components = cache.entity_components
        .entry(update.entity_id)
        .or_default();
    
    // Process changed components
    for (component_name, new_value) in update.changed_components {
        let component_state = entity_components
            .entry(component_name.clone())
            .or_insert_with(|| ComponentState {
                current_value: Value::Null,
                last_changed_time: current_time,
                change_indicator: ChangeIndicator::Added,
                previous_value: None,
            });
        
        // Check if value actually changed
        if component_state.current_value != new_value {
            component_state.previous_value = Some(component_state.current_value.clone());
            component_state.current_value = new_value;
            component_state.last_changed_time = current_time;
            component_state.change_indicator = ChangeIndicator::Changed { duration: 2.0 };
        }
    }
    
    // Process removed components
    for component_name in update.removed_components {
        if let Some(component_state) = entity_components.get_mut(&component_name) {
            component_state.change_indicator = ChangeIndicator::Removed;
            component_state.last_changed_time = current_time;
        }
    }
}
```

#### 3.2 Enhanced Component Viewer with Live Updates
**File:** `crates/bevy_dev_tools/src/inspector/ui/component_viewer.rs`

```rust
/// Enhanced component section creation with live update indicators
fn create_live_component_section(
    commands: &mut Commands,
    parent: Entity,
    entity_id: u32,
    component_name: &str,
    component_state: &ComponentState,
    current_time: f64,
) {
    let (category, display_name, full_path) = get_component_display_info(component_name);
    
    // Determine visual indicators based on change state
    let (background_color, border_color, indicator_text) = match &component_state.change_indicator {
        ChangeIndicator::Changed { duration } => {
            let age = current_time - component_state.last_changed_time;
            if age < *duration {
                let fade = (age / duration).min(1.0);
                let intensity = 1.0 - fade;
                (
                    Color::srgb(0.15 + intensity * 0.2, 0.15 + intensity * 0.1, 0.2), // Yellow-green fade
                    Color::srgb(0.3 + intensity * 0.4, 0.3 + intensity * 0.3, 0.4),
                    "🔄" // Changed indicator
                )
            } else {
                (Color::srgb(0.15, 0.15, 0.2), Color::srgb(0.3, 0.3, 0.4), "") // Normal
            }
        },
        ChangeIndicator::Added => (
            Color::srgb(0.15, 0.25, 0.15), // Green tint
            Color::srgb(0.3, 0.5, 0.3),
            "✨" // Added indicator
        ),
        ChangeIndicator::Removed => (
            Color::srgb(0.25, 0.15, 0.15), // Red tint
            Color::srgb(0.5, 0.3, 0.3),
            "❌" // Removed indicator
        ),
        ChangeIndicator::Unchanged => (
            Color::srgb(0.15, 0.15, 0.2), // Normal
            Color::srgb(0.3, 0.3, 0.4),
            ""
        ),
    };
    
    // Create section with visual change indicators
    let header_text = format!("{} {} [{}] {}", 
        if component_state.change_indicator != ChangeIndicator::Unchanged { "▶" } else { "▼" },
        display_name, 
        category,
        indicator_text
    );
    
    // Enhanced component section creation with styling based on change state
    let section_entity = commands.spawn((...)).id();
    
    // ... rest of component section creation with enhanced styling
}
```

### Phase 4: Integration and Performance Optimization

#### 4.1 System Integration
**File:** `crates/bevy_dev_tools/src/inspector/inspector.rs`

```rust
impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        app
            // ... existing resources ...
            .init_resource::<LiveComponentCache>()
            
            // ... existing systems ...
            .add_systems(Update, (
                // ... existing systems ...
                
                // New live update systems
                process_live_component_updates,
                update_live_component_viewer.after(process_live_component_updates),
                cleanup_expired_change_indicators,
                
                // Auto-start watching for selected entity
                auto_start_component_watching.after(handle_entity_selection),
            ));
    }
}

/// Automatically start watching components when entity is selected
pub fn auto_start_component_watching(
    mut http_client: ResMut<HttpRemoteClient>,
    selected_entity: Res<SelectedEntity>,
    entity_cache: Res<EntityCache>,
) {
    if !selected_entity.is_changed() {
        return;
    }
    
    if let Some(entity_id) = selected_entity.entity_id {
        if let Some(entity) = entity_cache.entities.get(&entity_id) {
            // Start watching all components of the selected entity
            let components: Vec<String> = entity.components.keys().cloned().collect();
            if !components.is_empty() {
                let _ = http_client.start_component_watching(entity_id, components);
            }
        }
    }
}
```

#### 4.2 Performance Optimizations

1. **Rate Limiting**: Update UI at 30 FPS maximum to prevent overwhelming
2. **Differential Updates**: Only update changed components in UI
3. **Memory Management**: Cleanup old change indicators and unused caches  
4. **Connection Pooling**: Reuse HTTP connections for watching multiple entities
5. **Lazy Loading**: Only start watching when component viewer is visible

### Phase 5: User Experience Enhancements

#### 5.1 Visual Indicators
- **Color-coded changes**: Green for additions, yellow for changes, red for removals
- **Fade animations**: Smooth transitions as change indicators fade out
- **Timestamps**: Show when components were last updated
- **Value diff display**: Show old vs new values for changed components

#### 5.2 Controls
- **Watch toggle**: Enable/disable live updates per component
- **Update frequency control**: Adjust refresh rate (10-60 FPS)
- **Pause/resume**: Temporarily stop updates for inspection
- **History view**: See recent changes to components

## Implementation Benefits

1. **No Third-Party Dependencies**: Uses only Bevy's built-in systems
2. **Efficient**: Leverages bevy_remote's change detection
3. **Scalable**: Can watch multiple entities and components simultaneously  
4. **Performant**: Rate-limited updates with differential rendering
5. **User-Friendly**: Clear visual indicators for all types of changes
6. **Robust**: Handles connection failures and automatically retries

## Implementation Order

1. **Phase 1**: Enhanced HTTP client with async streaming (1-2 days)
2. **Phase 2**: Component change detection and caching (1 day) 
3. **Phase 3**: Live UI updates with visual indicators (2 days)
4. **Phase 4**: System integration and performance optimization (1 day)
5. **Phase 5**: User experience polish and controls (1-2 days)

**Total Estimated Time: 6-8 days**

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
- bevy_tasks async processing is efficient for real-time updates
- Only changed components are sent (thanks to bevy_remote's change detection)
- Consider compression for large component data

## Dependencies Required

**Minimal dependencies** - using Bevy's built-in capabilities:
```toml
# Already available in Bevy:
# - bevy_tasks (for async processing)
# - bevy_remote (for watching functionality)
# - async_channel (for async communication)

# Only additional dependency needed:
reqwest = { version = "0.12", features = ["json"] } # Already in use
```

## Conclusion

This implementation plan leverages Bevy's existing capabilities to create a robust, efficient live component update system without external dependencies. The design is scalable, performant, and provides excellent user experience with clear visual feedback for all component changes.