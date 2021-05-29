// use bevy_app::{AppBuilder, Plugin};
// use bevy_utils::Duration;
// use bevy_xr::XrMode;
// use std::{cell::RefCell, rc::Rc, thread};
// use web_sys::{Closure, XrSession, XrSessionMode};

// #[derive(Clone)]
// struct WebXrContext {
//     pub session: XrSession,
//     pub mode: XrMode,
// }

// struct WebXrPlugin;

// impl Plugin for WebXrPlugin {
//     fn build(&self, app: &mut AppBuilder) {
//         let context = if let Some(context) = app.world().get_resource::<WebXrContext>() {
//             context.clone()
//         } else {
//             let system = web_sys::window().unwrap().navigator().xr();

//             // todo: this needs a better solution
//             let vr_supported = Rc::new(RefCell::new(None));
//             system
//                 .is_session_supported(XrSessionMode::ImmersiveVr)
//                 .then(Closure::wrap(Box::new({
//                     let vr_supported = Rc::clone(&vr_supported);
//                     move |res| {
//                         *vr_supported.borrow_mut() = Some(res);
//                     }
//                 })));
//             let ar_supported = Rc::new(RefCell::new(None));
//             system
//                 .is_session_supported(XrSessionMode::ImmersiveAr)
//                 .then(Closure::wrap(Box::new({
//                     let ar_supported = Rc::clone(&ar_supported);
//                     move |res| {
//                         *ar_supported.borrow_mut() = Some(res);
//                     }
//                 })));
//             let vr_supported = loop {
//                 if let Some(res) = vr_supported.borrow_mut().take() {
//                     break res;
//                 } else {
//                     thread::sleep(Duration::from_millis(10));
//                 }
//             };
//             let ar_supported = loop {
//                 if let Some(res) = vr_supported.borrow_mut().take() {
//                     break res;
//                 } else {
//                     thread::sleep(Duration::from_millis(10));
//                 }
//             };

//             let mode = app.world().get_resource::<XrMode>();
//             let session_mode = match mode {
//                 Some(XrMode::ImmersiveVR) | None if vr_supported => XrSessionMode::ImmersiveVr,
//                 Some(XrMode::ImmersiveAR) if ar_supported => XrSessionMode::ImmersiveAr,
//                 _ => XrSessionMode::Inline,
//             };

//             let new_mode = match session_mode {
//                 XrSessionMode::ImmersiveVr => XrMode::ImmersiveVR,
//                 XrSessionMode::ImmersiveAr => XrMode::ImmersiveAR,
//                 XrSessionMode::Inline => XrMode::InlineVR,
//             };

//             if let Some(mode) = mode {
//                 if new_mode != *mode {
//                     bevy_log::warn!("XrMode has been changed to {:?}", mode);
//                 }
//             }

//             let session = Rc::new(RefCell::new(None));
//             system
//                 .request_session(session_mode)
//                 .then(Closure::wrap(Box::new({
//                     let session = Rc::clone(&session);
//                     move |s: XrSession| {
//                         *session.borrow_mut() = Some(s);
//                     }
//                 })));
//             let session = loop {
//                 if let Some(session) = session.borrow_mut().take() {
//                     break session;
//                 } else {
//                     thread::sleep(Duration::from_millis(10));
//                 }
//             };

//             WebXrContext {
//                 session,
//                 mode: new_mode,
//             };
//         };
//         app.insert_resource(context.mode).insert_resource(context);
//     }
// }
