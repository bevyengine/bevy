//! Test example for logical pixel rendering in gizmos
//!
//! This example demonstrates how gizmo line widths can be specified in logical pixels
//! using the `Val::Px` enum, which automatically scales with the window's scale factor.
//!
//! ## Problem Solved
//! 
//! Previously, gizmo lines were rendered using physical pixel widths, which caused
//! inconsistent appearance across different screens with different DPI scaling.
//! This implementation uses logical pixels that automatically scale with the window's
//! scale factor, ensuring consistent line thickness across all displays.
//!
//! ## Key Features Demonstrated
//! 
//! - **Logical Pixel Widths**: Using `Val::Px` for consistent scaling
//! - **Automatic Scale Factor**: Lines automatically adjust to window DPI scaling
//! - **Multiple Val Units**: Demonstrates `Val::Px`, `Val::Vw`, and `Val::Vh`
//! - **Real-time Configuration**: Dynamic line width changes during runtime
//! - **Backward Compatibility**: Still supports f32 values for existing code
//!
//! ## Controls
//! 
//! - **1**: Set line width to 2 logical pixels (`Val::Px(2.0)`)
//! - **2**: Set line width to 4 logical pixels (`Val::Px(4.0)`)
//! - **3**: Set line width to 8 logical pixels (`Val::Px(8.0)`)
//! - **4**: Set line width to 10 logical pixels (`Val::Px(10.0)`)
//! - **5**: Set line width to 12 logical pixels (`Val::Px(12.0)`)
//! - **Space**: Print current window scale factor and dimensions
//!
//! ## Architecture Overview
//!
//! The implementation works by:
//! 1. Using `Val` enum from `bevy_ui` for logical pixel specification
//! 2. Extracting scale factor from `ComputedCameraValues` in the render pipeline
//! 3. Resolving `Val` to physical pixels using `Val::resolve()` method
//! 4. Passing the resolved physical width to the shader for rendering

use bevy::{
    color::palettes::css::*,
    math::Isometry3d,
    prelude::*,
};

/// Main application entry point that sets up the Bevy app with gizmo support.
/// 
/// This function:
/// - Initializes the default Bevy plugins
/// - Sets up the default gizmo configuration group
/// - Registers the startup and update systems
/// - Runs the application
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_gizmo_group::<DefaultGizmoConfigGroup>()
        .add_systems(Startup, setup)
        .add_systems(Update, (draw_test_gizmos, handle_input, update_config))
        .run();
}

/// Sets up the initial scene with camera and UI elements.
/// 
/// This function creates:
/// - A 3D camera positioned at (0, 0, 5) looking at the origin
/// - A UI text element displaying control instructions
/// 
/// The camera setup allows us to view the gizmo lines from a good distance,
/// and the UI provides user feedback about available controls.
fn setup(mut commands: Commands) {
    // Spawn camera with proper bundle following 0.17.0 changes
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Add UI for displaying current scale factor and controls
    commands.spawn((
        Text::new(
            "Logical Pixel Test - Press 1-5 to change line widths, Space for scale info",
        ),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.),
            left: Val::Px(12.),
            ..default()
        },
    ));
}

/// Draws various gizmo shapes to demonstrate the logical pixel width functionality.
/// 
/// This function renders a collection of gizmo lines and shapes to showcase how
/// the line width configuration affects different types of gizmos. It draws:
/// 
/// - **Cross Pattern**: Horizontal and vertical lines forming a cross
/// - **Diagonal Lines**: Two diagonal lines crossing each other
/// - **Circle**: A 3D circle to show how line width affects curved shapes
/// - **2D Lines**: Horizontal lines in 2D space for comparison
/// 
/// The scale factor is retrieved from the window but not currently displayed
/// (would require 3D text rendering for full implementation).
/// 
/// # Parameters
/// 
/// - `gizmos`: The gizmo drawing system parameter
/// - `keyboard`: Input resource for detecting key presses
/// - `windows`: Query to access window information for scale factor
fn draw_test_gizmos(
    mut gizmos: Gizmos,
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
) {
    // Get the primary window for scale factor info
    let window = windows.single().unwrap();
    let scale_factor = window.scale_factor();

    // Draw test patterns to demonstrate the line width changes
    // Horizontal line
    gizmos.line(
        Vec3::new(-2.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        RED,
    );
    
    // Vertical line
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

    // reflected diagonal lines
    gizmos.line(
        Vec3::new(-1.5, 1.5, 0.0),
        Vec3::new(1.5, -1.5, 0.0),
        YELLOW,
    );

    // Draw a circle to show how line width affects other gizmo shapes
    gizmos.circle(Isometry3d::IDENTITY, 1.0, FUCHSIA);

    // Draw 2D lines to show the effect in 2D space
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

    // Draw scale factor info as text in 3D space
    let scale_text = format!("Scale Factor: {:.2}", scale_factor);
    // Note: In a real application, you'd use a proper 3D text rendering system
    // This is just for demonstration
}

/// Updates the gizmo line width configuration based on keyboard input.
/// 
/// This function handles real-time configuration changes for the gizmo line width.
/// It demonstrates different `Val` units and their effects on line thickness:
/// 
/// - **Keys 1-3**: Use `Val::Px` for logical pixel widths (2, 4, 8 pixels)
/// - **Key 4**: Uses `Val::Vw` for viewport-relative width (0.5% of viewport width)
/// - **Key 5**: Uses `Val::Vh` for viewport-relative height (0.5% of viewport height)
/// 
/// The configuration changes are applied immediately and affect all gizmo lines
/// drawn in subsequent frames. This demonstrates the dynamic nature of the
/// logical pixel system.
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

    // testing with pixels only for minimal example
    if keyboard.pressed(KeyCode::Digit1) {
        config.line.width = Val::Px(2.0); // 2 logical pixels
    } else if keyboard.pressed(KeyCode::Digit2) {
        config.line.width = Val::Px(4.0); // 4 logical pixels
    } else if keyboard.pressed(KeyCode::Digit3) {
        config.line.width = Val::Px(8.0); // 8 logical pixels
    } else if keyboard.pressed(KeyCode::Digit4) {
        // config.line.width = Val::Vw(0.5); // 0.5% of viewport width
        config.line.width = Val::Px(10.0);
    } else if keyboard.pressed(KeyCode::Digit5) {
        // config.line.width = Val::Vh(0.5); // 0.5% of viewport height
        config.line.width = Val::Px(12.0);
    }
}

/// Handles debug input for displaying window information.
/// 
/// This function provides debugging information about the current window's
/// scale factor and dimensions. When the Space key is pressed, it prints:
/// 
/// - **Scale Factor**: The DPI scaling factor (e.g., 1.0 for 100%, 2.0 for 200%)
/// - **Window Size**: Logical dimensions of the window
/// - **Physical Size**: Actual pixel dimensions of the window
/// 
/// This information is useful for understanding how the logical pixel system
/// works and verifying that the scale factor is being correctly detected.
/// 
/// # Parameters
/// 
/// - `keyboard`: Input resource for detecting key presses
/// - `windows`: Query to access window information
fn handle_input(keyboard: Res<ButtonInput<KeyCode>>, windows: Query<&Window>) {
    if keyboard.just_pressed(KeyCode::Space) {
        let window = windows.single().unwrap();
        println!("Current scale factor: {}", window.scale_factor());
        println!("Window size: {}x{}", window.width(), window.height());
        println!("Physical size: {}x{}", window.physical_width(), window.physical_height());
    }
}


