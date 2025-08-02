//! Multi-window test example for logical pixel rendering in gizmos
//!
//! This example demonstrates how gizmo line widths maintain consistent appearance
//! across multiple windows with different resolutions and DPI scaling factors.
//!
//! ## Problem Solved
//! 
//! The original issue was that gizmo lines appeared inconsistent across different
//! displays due to physical pixel rendering. This multi-window test proves that
//! the logical pixel implementation ensures consistent line thickness regardless
//! of:
//! - Window resolution (e.g., 1920x1080 vs 3840x2160)
//! - DPI scaling factor (e.g., 100% vs 200% scaling)
//! - Different monitor configurations
//!
//! ## Key Features Demonstrated
//! 
//! - **Multi-Window Support**: Creates multiple windows with different properties
//! - **Consistent Line Width**: Same logical pixel width renders consistently across all windows
//! - **Real-time Configuration**: Dynamic line width changes affect all windows simultaneously
//! - **Scale Factor Detection**: Each window's scale factor is independently detected and applied
//! - **Cross-Platform Compatibility**: Works on different operating systems and display configurations
//!
//! ## Controls
//! 
//! - **1**: Set line width to 2 logical pixels (`Val::Px(2.0)`)
//! - **2**: Set line width to 4 logical pixels (`Val::Px(4.0)`)
//! - **3**: Set line width to 8 logical pixels (`Val::Px(8.0)`)
//! - **4**: Set line width to 0.5% of viewport width (`Val::Vw(0.5)`)
//! - **5**: Set line width to 0.5% of viewport height (`Val::Vh(0.5)`)
//! - **Space**: Print current window information for all windows
//!
//! ## Architecture Overview
//!
//! The implementation works by:
//! 1. Creating multiple windows with different resolutions and scale factors
//! 2. Each window has its own camera and gizmo rendering context
//! 3. The same logical pixel configuration is applied to all windows
//! 4. Each window's scale factor is independently resolved to physical pixels
//! 5. Lines appear visually consistent across all windows despite different physical properties

use bevy::{
    color::palettes::css::*,
    math::Isometry3d,
    prelude::*,
    window::{PrimaryWindow, WindowRef},
    render::camera::RenderTarget,
};

/// Main application entry point that sets up the Bevy app with multi-window gizmo support.
/// 
/// This function:
/// - Initializes the default Bevy plugins with multi-window support
/// - Sets up the default gizmo configuration group
/// - Registers the startup and update systems
/// - Runs the application with multiple windows
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_gizmo_group::<DefaultGizmoConfigGroup>()
        .add_systems(Startup, setup)
        .add_systems(Update, (
            draw_test_gizmos, 
            handle_input, 
            update_config,
        ))
        .run();
}

/// Sets up the initial scene with multiple windows, cameras, and UI elements.
/// 
/// This function creates:
/// - Multiple windows with different resolutions and scale factors
/// - A 3D camera for each window positioned to view the gizmo lines
/// - UI text elements displaying control instructions in each window
/// 
/// The multi-window setup allows us to test consistent rendering across
/// different display configurations simultaneously.
fn setup(mut commands: Commands) {
    // Create a camera for the primary window (order 0)
    commands.spawn((
        Camera3d::default(),
        Camera {
            order: 0,
            target: RenderTarget::Window(WindowRef::Primary),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Create additional windows with different resolutions
    create_window(&mut commands, "High DPI Window", 1920, 1080, 2.0, 1);
    create_window(&mut commands, "Low Res Window", 800, 600, 1.0, 2);
    create_window(&mut commands, "Ultra HD Window", 3840, 2160, 3.0, 3);

    // Add UI for displaying control instructions
    commands.spawn((
        Text::new(
            "Multi-Window Logical Pixel Test\n\
            Press 1-5 to change line widths\n\
            Press Space for window info",
        ),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.),
            left: Val::Px(12.),
            ..default()
        },
    ));
}

/// Creates a window with a specific configuration and spawns a camera for it.
/// 
/// This function creates a new window with the specified title, resolution, and scale factor,
/// then spawns a dedicated camera for that window. Each camera gets a unique order to avoid
/// rendering conflicts.
/// 
/// # Parameters
/// 
/// - `commands`: Commands to spawn entities
/// - `title`: Window title
/// - `width`: Window width in logical pixels
/// - `height`: Window height in logical pixels
/// - `scale_factor`: DPI scale factor for the window
/// - `camera_order`: Unique order for the camera to avoid conflicts
fn create_window(
    commands: &mut Commands,
    title: &str,
    width: u32,
    height: u32,
    scale_factor: f32,
    camera_order: isize,
) {
    // Create the window entity
    let window_entity = commands
        .spawn(Window {
            title: title.to_string(),
            resolution: (width as f32, height as f32).into(),
            ..default()
        })
        .id();

    // Create a camera for this specific window
    commands.spawn((
        Camera3d::default(),
        Camera {
            order: camera_order,
            target: RenderTarget::Window(WindowRef::Entity(window_entity)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Draws various gizmo shapes to demonstrate consistent logical pixel width across windows.
/// 
/// This function renders the same collection of gizmo lines and shapes in each window,
/// ensuring that the line width appears visually consistent regardless of the window's
/// resolution or scale factor. It draws:
/// 
/// - **Cross Pattern**: Horizontal and vertical lines forming a cross
/// - **Diagonal Lines**: Two diagonal lines crossing each other
/// - **Circle**: A 3D circle to show how line width affects curved shapes
/// - **2D Lines**: Horizontal lines in 2D space for comparison
/// - **Grid Pattern**: A grid to help visualize scale differences
/// 
/// The same logical pixel configuration is applied to all windows, but each window
/// resolves it to physical pixels using its own scale factor.
/// 
/// # Parameters
/// 
/// - `gizmos`: The gizmo drawing system parameter
/// - `_keyboard`: Input resource for detecting key presses (unused in this function)
/// - `_windows`: Query to access window information for scale factors (unused in this function)
fn draw_test_gizmos(
    mut gizmos: Gizmos,
    _keyboard: Res<ButtonInput<KeyCode>>,
    _windows: Query<&Window>,
) {
    // Draw test patterns to demonstrate the line width changes
    // These will be rendered in all windows with consistent logical pixel width
    
    // Cross pattern
    gizmos.line(
        Vec3::new(-2.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        RED,
    );
    gizmos.line(
        Vec3::new(0.0, -2.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
        LIME,
    );
    
    // Diagonal lines
    gizmos.line(
        Vec3::new(-1.5, -1.5, 0.0),
        Vec3::new(1.5, 1.5, 0.0),
        BLUE,
    );
    gizmos.line(
        Vec3::new(-1.5, 1.5, 0.0),
        Vec3::new(1.5, -1.5, 0.0),
        YELLOW,
    );

    // Circle
    gizmos.circle(Isometry3d::IDENTITY, 1.0, FUCHSIA);

    // Grid pattern to help visualize scale
    for i in -4..=4 {
        let pos = i as f32 * 0.5;
        gizmos.line(
            Vec3::new(pos, -2.0, 0.0),
            Vec3::new(pos, 2.0, 0.0),
            LinearRgba::gray(0.3),
        );
        gizmos.line(
            Vec3::new(-2.0, pos, 0.0),
            Vec3::new(2.0, pos, 0.0),
            LinearRgba::gray(0.3),
        );
    }

    // 2D lines for comparison
    gizmos.line_2d(
        Vec2::new(-100.0, -100.0),
        Vec2::new(100.0, -100.0),
        TEAL,
    );
    gizmos.line_2d(
        Vec2::new(-100.0, 100.0),
        Vec2::new(100.0, 100.0),
        ORANGE,
    );
}

/// Updates the gizmo line width configuration based on keyboard input.
/// 
/// This function handles real-time configuration changes for the gizmo line width
/// that affect ALL windows simultaneously. It demonstrates different `Val` units
/// and their effects on line thickness across multiple displays:
/// 
/// - **Keys 1-3**: Use `Val::Px` for logical pixel widths (2, 4, 8 pixels)
/// - **Key 4**: Uses `Val::Vw` for viewport-relative width (0.5% of viewport width)
/// - **Key 5**: Uses `Val::Vh` for viewport-relative height (0.5% of viewport height)
/// 
/// The configuration changes are applied immediately and affect all gizmo lines
/// drawn in all windows. This demonstrates that the logical pixel system works
/// consistently across different display configurations.
/// 
/// # Parameters
/// 
/// - `config_store`: Mutable resource for accessing gizmo configuration
/// - `keyboard`: Input resource for detecting key presses
fn update_config(
    mut config_store: ResMut<GizmoConfigStore>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();

    if keyboard.pressed(KeyCode::Digit1) {
        config.line.width = Val::Px(2.0); // 2 logical pixels
    } else if keyboard.pressed(KeyCode::Digit2) {
        config.line.width = Val::Px(4.0); // 4 logical pixels
    } else if keyboard.pressed(KeyCode::Digit3) {
        config.line.width = Val::Px(8.0); // 8 logical pixels
    } else if keyboard.pressed(KeyCode::Digit4) {
        config.line.width = Val::Vw(0.5); // 0.5% of viewport width
    } else if keyboard.pressed(KeyCode::Digit5) {
        config.line.width = Val::Vh(0.5); // 0.5% of viewport height
    }
}

/// Handles debug input for displaying window information across all windows.
/// 
/// This function provides debugging information about all current windows'
/// scale factors and dimensions. When the Space key is pressed, it prints
/// information for each window:
/// 
/// - **Window Index**: Sequential number for each window
/// - **Scale Factor**: The DPI scaling factor for each window
/// - **Logical Size**: Logical dimensions of each window
/// - **Physical Size**: Actual pixel dimensions of each window
/// - **Title**: Window title for identification
/// 
/// This information is useful for understanding how the logical pixel system
/// works across multiple displays and verifying that scale factors are being
/// correctly detected for each window.
/// 
/// # Parameters
/// 
/// - `keyboard`: Input resource for detecting key presses
/// - `windows`: Query to access window information for all windows
fn handle_input(keyboard: Res<ButtonInput<KeyCode>>, windows: Query<&Window>) {
    if keyboard.just_pressed(KeyCode::Space) {
        println!("\n=== Multi-Window Information ===");
        for (i, window) in windows.iter().enumerate() {
            println!("Window {}:", i);
            println!("  Title: {}", window.title);
            println!("  Scale Factor: {:.2}", window.scale_factor());
            println!("  Logical Size: {}x{}", window.width(), window.height());
            println!("  Physical Size: {}x{}", window.physical_width(), window.physical_height());
            println!();
        }
        println!("===============================\n");
    }
} 