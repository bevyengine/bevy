use super::util::{call_hal, ffx_check_result};
use bevy_core::FrameCount;
use bevy_ecs::system::Resource;
use bevy_math::UVec2;
use bevy_render::{
    camera::TemporalJitter,
    render_resource::{CommandBuffer, CommandEncoderDescriptor},
    renderer::RenderDevice,
};
use fsr::*;
use std::mem::MaybeUninit;

#[derive(Resource)]
pub struct FsrManager {
    interface: FfxFsr2Interface,
    context: Option<FfxFsr2Context>,
    _scratch_buffer: Box<[u8]>,
    render_device: RenderDevice,

    current_context_max_input_resolution: UVec2,
    current_context_max_upscaled_resolution: UVec2,
    current_context_hdr: bool,
}

impl FsrManager {
    pub fn new(render_device: RenderDevice) -> Option<Self> {
        let r = render_device.clone();
        call_hal(&render_device, |device| {
            let physical_device = device.raw_physical_device();
            let get_device_proc_addr = device
                .shared_instance()
                .raw_instance()
                .fp_v1_0()
                .get_device_proc_addr;

            let scratch_buffer_size = unsafe { ffxFsr2GetScratchMemorySizeVK(physical_device) };
            let mut _scratch_buffer = vec![0_u8; scratch_buffer_size].into_boxed_slice();

            let mut interface = MaybeUninit::<FfxFsr2Interface>::uninit();
            ffx_check_result(unsafe {
                ffxFsr2GetInterfaceVK(
                    interface.as_mut_ptr(),
                    _scratch_buffer.as_mut_ptr() as *mut _,
                    scratch_buffer_size,
                    physical_device,
                    get_device_proc_addr,
                )
            })?;
            let interface = unsafe { interface.assume_init() };

            Some(Self {
                interface,
                context: None,
                _scratch_buffer,
                render_device: r,
                current_context_max_input_resolution: UVec2::ZERO,
                current_context_max_upscaled_resolution: UVec2::ZERO,
                current_context_hdr: false,
            })
        })
    }

    pub fn recreate_context_if_needed(
        &mut self,
        max_input_resolution: UVec2,
        max_upscaled_resolution: UVec2,
        hdr: bool,
    ) {
        if max_input_resolution.x > self.current_context_max_input_resolution.x
            || max_input_resolution.y > self.current_context_max_input_resolution.y
            || max_upscaled_resolution.x > self.current_context_max_upscaled_resolution.x
            || max_upscaled_resolution.y > self.current_context_max_upscaled_resolution.y
            || hdr != self.current_context_hdr
        {
            self.current_context_max_input_resolution = max_input_resolution;
            self.current_context_max_upscaled_resolution = max_upscaled_resolution;
            self.current_context_hdr = hdr;
        } else {
            return;
        }

        self.destroy_context();

        call_hal(&self.render_device, |device| {
            let mut flags = FfxFsr2InitializationFlagBits_FFX_FSR2_ENABLE_DEPTH_INFINITE
                | FfxFsr2InitializationFlagBits_FFX_FSR2_ENABLE_DEPTH_INVERTED;
            if hdr {
                flags |= FfxFsr2InitializationFlagBits_FFX_FSR2_ENABLE_HIGH_DYNAMIC_RANGE;
            }

            let context_description = FfxFsr2ContextDescription {
                flags,
                maxRenderSize: FfxDimensions2D {
                    width: max_input_resolution.x,
                    height: max_input_resolution.y,
                },
                displaySize: FfxDimensions2D {
                    width: max_upscaled_resolution.x,
                    height: max_upscaled_resolution.y,
                },
                callbacks: self.interface,
                device: unsafe { ffxGetDeviceVK(device.raw_device().handle()) },
            };

            let mut context = MaybeUninit::<FfxFsr2Context>::uninit();
            ffx_check_result(unsafe {
                ffxFsr2ContextCreate(context.as_mut_ptr(), &context_description)
            })?;
            self.context = Some(unsafe { context.assume_init() });

            Some(())
        })
        .expect("Failed to create FSR context");
    }

    pub fn get_temporal_jitter(
        &mut self,
        input_resolution: UVec2,
        upscaled_resolution: UVec2,
        frame_count: FrameCount,
    ) -> TemporalJitter {
        let phase_count = unsafe {
            ffxFsr2GetJitterPhaseCount(input_resolution.x as i32, upscaled_resolution.x as i32)
        };

        let mut temporal_jitter = TemporalJitter::default();
        ffx_check_result(unsafe {
            ffxFsr2GetJitterOffset(
                &mut temporal_jitter.offset.x,
                &mut temporal_jitter.offset.y,
                frame_count.0 as i32,
                phase_count,
            )
        })
        .expect("Failed to get FSR temporal jitter");

        temporal_jitter
    }

    pub fn record_command_buffer(&mut self, resources: FsrResources) -> CommandBuffer {
        let command_encoder = self
            .render_device
            .create_command_encoder(&CommandEncoderDescriptor { label: Some("fsr") });

        let dispatch_description = FfxFsr2DispatchDescription {
            commandList: todo!(),
            color: todo!(),
            depth: todo!(),
            motionVectors: todo!(),
            exposure: todo!(),
            reactive: todo!(),
            transparencyAndComposition: todo!(),
            output: todo!(),
            jitterOffset: todo!(),
            motionVectorScale: todo!(),
            renderSize: todo!(),
            enableSharpening: todo!(),
            sharpness: todo!(),
            frameTimeDelta: todo!(),
            preExposure: todo!(),
            reset: todo!(),
            cameraNear: todo!(),
            cameraFar: todo!(),
            cameraFovAngleVertical: todo!(),
        };

        let context = self.context.as_mut().expect("FSR context does not exist");

        unsafe { ffx_check_result(ffxFsr2ContextDispatch(context, &dispatch_description)) }
            .expect("Failed to render FSR");

        command_encoder.finish()
    }

    fn destroy_context(&mut self) {
        if let Some(mut context) = self.context.take() {
            call_hal(&self.render_device, |device| {
                unsafe { device.raw_device().device_wait_idle() }
                    .expect("Failed to wait for GPU to be idle");

                ffx_check_result(unsafe { ffxFsr2ContextDestroy(&mut context) })
            })
            .expect("Failed to destroy FSR context");
        }
    }
}

impl Drop for FsrManager {
    fn drop(&mut self) {
        self.destroy_context();
    }
}

unsafe impl Send for FsrManager {}
unsafe impl Sync for FsrManager {}

pub struct FsrResources {}
