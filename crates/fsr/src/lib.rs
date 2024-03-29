#![allow(improper_ctypes)] // https://github.com/rust-lang/rust-bindgen/issues/1549
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(rustdoc::broken_intra_doc_links)]
#![allow(unused)]

include!("bindings.rs");

type PFN_vkGetDeviceProcAddr = ash::vk::PFN_vkGetDeviceProcAddr;
type VkBuffer = ash::vk::Buffer;
type VkCommandBuffer = ash::vk::CommandBuffer;
type VkDevice = ash::vk::Device;
pub type VkFormat = ash::vk::Format;
pub type VkImage = ash::vk::Image;
pub type VkImageView = ash::vk::ImageView;
type VkPhysicalDevice = ash::vk::PhysicalDevice;
