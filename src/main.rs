use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;
use std::ffi::{c_void, CStr, CString};
use std::ptr;

#[cfg(target_os = "windows")]
use ash::extensions::khr::Win32Surface;

use ash::extensions::ext::DebugUtils;
use ash::extensions::khr::Surface;

const WINDOW_TITLE: &str = "01 instance creation";
const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

pub const APPLICATION_VERSION: u32 = 1;
pub const ENGINE_VERSION: u32 = 1;

fn u8_to_string(i8_str: &[i8]) -> String {
    let ptr = i8_str.as_ptr();
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .expect("Failed to convert vulkan raw pointer")
        .to_owned()
}

#[cfg(all(windows))]
pub fn required_extension_names() -> Vec<*const i8> {
    vec![
        Surface::name().as_ptr(),
        Win32Surface::name().as_ptr(),
        DebugUtils::name().as_ptr(),
    ]
}

unsafe extern "system" fn vulkan_debug_utils_debug(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    p_use_data: *mut c_void,
) -> vk::Bool32 {
    let message_severity_str = match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => "[Verbose]",
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => "[Warning]",
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => "[Error]",
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => "[Info]",
        _ => "[Unknown]",
    };

    let message_type_str = match message_type {
        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "[General]",
        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[Performance]",
        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[Validation]",
        _ => "[Unknown]",
    };

    let message = unsafe { CStr::from_ptr((*p_callback_data).p_message) };

    if message_severity == vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
        println!(
            "[Debug]{}{}{:?}",
            message_severity_str, message_type_str, message
        );
    }

    vk::FALSE
}

pub fn check_validation_layer_support(entry: &ash::Entry, layers: &[&'static str]) -> bool {
    let layer_properties = entry
        .enumerate_instance_layer_properties()
        .expect("Failed to enumerate Instance Layers Properties");

    for check_layer in layers.iter() {
        let mut found = false;
        for property in layer_properties.iter() {
            let c_str = u8_to_string(&property.layer_name);

            if c_str == *check_layer {
                found = true;
                break;
            }
        }

        if !found {
            println!("Failed to find layer {}", *check_layer);
            return false;
        }
    }
    return true;
}

fn get_debug_utils_messenger_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT {
    vk::DebugUtilsMessengerCreateInfoEXT {
        s_type: vk::StructureType::DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
        p_next: ptr::null(),
        flags: vk::DebugUtilsMessengerCreateFlagsEXT::empty(),
        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
            | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
            | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
        message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
            | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
            | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
        pfn_user_callback: Some(vulkan_debug_utils_debug),
        p_user_data: ptr::null_mut(),
    }
}

fn get_debug_messenger(
    create_info: &vk::DebugUtilsMessengerCreateInfoEXT,
    debug_utils_loader: &ash::extensions::ext::DebugUtils,
) -> vk::DebugUtilsMessengerEXT {
    if !VALIDATION_INFO.enable_validation {
        vk::DebugUtilsMessengerEXT::null()
    } else {
        let utils_messenger = unsafe {
            debug_utils_loader
                .create_debug_utils_messenger(&create_info, None)
                .expect("Failed to set up debug messenger!")
        };

        utils_messenger
    }
}

fn get_require_layer_raw_names() -> Vec<*const i8> {
    if VALIDATION_INFO.enable_validation {
        VALIDATION_INFO
            .required_validation_layers
            .iter()
            .map(|layer_name| *layer_name as *const str as *const i8)
            .collect::<Vec<*const i8>>()
    } else {
        Vec::new()
    }
}

fn print_physical_device_info(instance: &ash::Instance, p_device: vk::PhysicalDevice) {
    let p_device_properties = unsafe { instance.get_physical_device_properties(p_device) };
    let p_device_features = unsafe { instance.get_physical_device_features(p_device) };
    let p_device_queue_families =
        unsafe { instance.get_physical_device_queue_family_properties(p_device) };

    // ??????gpu????????????
    let device_type = match p_device_properties.device_type {
        vk::PhysicalDeviceType::CPU => "CPU",
        vk::PhysicalDeviceType::INTEGRATED_GPU => "Integerate GPU",
        vk::PhysicalDeviceType::DISCRETE_GPU => "Discrete GPU",
        vk::PhysicalDeviceType::VIRTUAL_GPU => "Virtual GPU",
        vk::PhysicalDeviceType::OTHER => "Unknown",
        _ => panic!(),
    };

    let device_name = u8_to_string(&p_device_properties.device_name);
    println!(
        "\tDevice Name: {}, id: {}, type: {}",
        device_name, p_device_properties.device_id, device_type
    );

    println!("\tAPI Version: {}", p_device_properties.api_version);

    println!("\tSupport Queue Family: {}", p_device_queue_families.len());
    println!("\t\tQueue Count | Graphics, Compute, Transfer, Sparse Binding");
    for queue_family in p_device_queue_families.iter() {
        let is_graphics_support = if queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
            "support"
        } else {
            "unsupport"
        };
        let is_compute_support = if queue_family.queue_flags.contains(vk::QueueFlags::COMPUTE) {
            "support"
        } else {
            "unsupport"
        };
        let is_transfer_support = if queue_family.queue_flags.contains(vk::QueueFlags::TRANSFER) {
            "support"
        } else {
            "unsupport"
        };
        let is_sparse_support = if queue_family
            .queue_flags
            .contains(vk::QueueFlags::SPARSE_BINDING)
        {
            "support"
        } else {
            "unsupport"
        };

        println!(
            "\t\t{}\t    | {},  {},  {},  {}",
            queue_family.queue_count,
            is_graphics_support,
            is_compute_support,
            is_transfer_support,
            is_sparse_support
        );
    }
}

fn find_queue_family(
    instance: &ash::Instance,
    p_device: vk::PhysicalDevice,
    surface_stuff: &SurfaceStuff,
) -> QueueFamilyIndices {
    let p_device_queue_families =
        unsafe { instance.get_physical_device_queue_family_properties(p_device) };
    let mut indices: QueueFamilyIndices = QueueFamilyIndices {
        graphics_family: None,
        present_family: None,
    };

    let mut index = 0u32;
    // ????????????
    for queue_family in p_device_queue_families.iter() {
        let is_graphics_support = queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS);
        let is_present_support = unsafe {
            surface_stuff
                .surface_loader
                .get_physical_device_surface_support(p_device, index, surface_stuff.surface_khr)
                .expect("Failed to get physic device surface support")
        };
        // let is_compute_support = queue_family.queue_flags.contains(vk::QueueFlags::COMPUTE);
        // let is_tranfer_suppoprt = queue_family.queue_flags.contains(vk::QueueFlags::TRANSFER);
        if queue_family.queue_count > 0 {
            if is_graphics_support {
                indices.graphics_family = Some(index);
            }

            if is_present_support {
                indices.present_family = Some(index);
            }
        }

        if indices.is_complete() {
            break;
        }

        index += 1;
    }

    indices
}

fn check_physic_device_extension_support(
    instance: &ash::Instance,
    p_device: vk::PhysicalDevice,
) -> bool {
    let avaliable_extensions = unsafe {
        instance
            .enumerate_device_extension_properties(p_device)
            .expect("Failed to get physical device extension properties")
    };

    let mut required_ext_set = std::collections::HashSet::new();

    for ext in DEVICE_EXTENSIONS.name {
        required_ext_set.insert(ext.to_string());
    }

    for aval_ext in avaliable_extensions.iter() {
        let aval_ext_name = u8_to_string(&aval_ext.extension_name);
        required_ext_set.remove(&aval_ext_name);
    }

    required_ext_set.is_empty()
}

fn is_device_suitable(
    instance: &ash::Instance,
    p_device: vk::PhysicalDevice,
    surface_stuff: &SurfaceStuff,
) -> bool {
    let queue_family_indices = find_queue_family(instance, p_device, surface_stuff);

    let extensions_support = check_physic_device_extension_support(instance, p_device);

    let mut swap_chain_adequate = false;
    if extensions_support {
        let swap_chain_sd = query_swap_chain_support(instance, surface_stuff, p_device);
        swap_chain_adequate =
            !swap_chain_sd.formats.is_empty() && !swap_chain_sd.present_modes.is_empty();
    }

    return queue_family_indices.is_complete() && extensions_support && swap_chain_adequate;
}

fn pick_physic_device(
    instance: &ash::Instance,
    surface_stuff: &SurfaceStuff,
) -> vk::PhysicalDevice {
    let physical_devices = unsafe {
        instance
            .enumerate_physical_devices()
            .expect("Failed to enumerate Physical Devices!")
    };

    if physical_devices.len() == 0 {
        panic!("Failed to find GPUs with vulkan support.");
    }

    println!(
        "{} devices (GPU) found with vulkan support.",
        physical_devices.len()
    );

    let mut suitable_device = None;
    for &device in physical_devices.iter() {
        if is_device_suitable(instance, device, surface_stuff) {
            suitable_device = Some(device);
        }
    }

    match suitable_device {
        Some(deivce) => deivce,
        None => panic!("Failed to find a suitable GPU!"),
    }
}

fn create_logic_device(
    instance: &ash::Instance,
    p_device: vk::PhysicalDevice,
    queue_family_indices: &QueueFamilyIndices,
) -> ash::Device {
    let mut unique_queue_familes = std::collections::HashSet::new();
    unique_queue_familes.insert(queue_family_indices.graphics_family.unwrap());
    unique_queue_familes.insert(queue_family_indices.present_family.unwrap());
    let mut device_queue_create_infos = Vec::new();
    for index in unique_queue_familes.iter() {
        let queue_priority = [1.0f32];
        let device_queue_ci = vk::DeviceQueueCreateInfo {
            s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::DeviceQueueCreateFlags::empty(),
            queue_family_index: *index,
            queue_count: queue_priority.len() as u32,
            p_queue_priorities: queue_priority.as_ptr(),
        };
        device_queue_create_infos.push(device_queue_ci);
    }

    let require_layer_raw_names = get_require_layer_raw_names();

    let device_features = vk::PhysicalDeviceFeatures {
        ..Default::default()
    };

    let enable_extension_names = [
        ash::extensions::khr::Swapchain::name().as_ptr(), // currently just enable the Swapchain extension.
    ];

    let device_ci = vk::DeviceCreateInfo {
        s_type: vk::StructureType::DEVICE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::DeviceCreateFlags::empty(),
        queue_create_info_count: 1,
        p_queue_create_infos: device_queue_create_infos.as_ptr(),
        enabled_layer_count: require_layer_raw_names.len() as u32,
        pp_enabled_layer_names: require_layer_raw_names.as_ptr(),
        enabled_extension_count: enable_extension_names.len() as u32,
        pp_enabled_extension_names: enable_extension_names.as_ptr(),
        p_enabled_features: &device_features,
    };

    unsafe {
        instance
            .create_device(p_device, &device_ci, None)
            .expect("Failed to create logical device!")
    }
}

pub struct ValidationInfo {
    pub enable_validation: bool,
    pub required_validation_layers: [&'static str; 1],
}

pub struct DeviceExtension {
    pub name: [&'static str; 1],
}

pub struct QueueFamilyIndices {
    graphics_family: Option<u32>,
    present_family: Option<u32>,
}

impl QueueFamilyIndices {
    pub fn is_complete(&self) -> bool {
        return self.graphics_family.is_some() && self.present_family.is_some();
    }
}

pub struct SwapChainSupportDetails {
    capabilities: vk::SurfaceCapabilitiesKHR,
    formats: Vec<vk::SurfaceFormatKHR>,
    present_modes: Vec<vk::PresentModeKHR>,
}

pub struct SwapChainStuff {
    swapchain_loader: ash::extensions::khr::Swapchain,
    swapchain_khr: vk::SwapchainKHR,
    swapchain_format: vk::Format,
    swapchain_extent: vk::Extent2D,
    swapchain_image: Vec<vk::Image>,
}

fn query_swap_chain_support(
    instance: &ash::Instance,
    surface_stuff: &SurfaceStuff,
    p_device: vk::PhysicalDevice,
) -> SwapChainSupportDetails {
    let capabilities = unsafe {
        surface_stuff
            .surface_loader
            .get_physical_device_surface_capabilities(p_device, surface_stuff.surface_khr)
            .expect("Failed to query for surface capabilities.")
    };
    let formats = unsafe {
        surface_stuff
            .surface_loader
            .get_physical_device_surface_formats(p_device, surface_stuff.surface_khr)
            .expect("Failed to query for surface formats.")
    };
    let present_modes = unsafe {
        surface_stuff
            .surface_loader
            .get_physical_device_surface_present_modes(p_device, surface_stuff.surface_khr)
            .expect("Failed to query for surface present modes.")
    };

    SwapChainSupportDetails {
        capabilities,
        formats,
        present_modes,
    }
}

fn choose_swap_surface_format(
    avaliable_formats: &Vec<vk::SurfaceFormatKHR>,
) -> vk::SurfaceFormatKHR {
    for format in avaliable_formats {
        if format.format == vk::Format::B8G8R8A8_SRGB
            && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        {
            return format.clone();
        }
    }

    avaliable_formats.first().unwrap().clone()
}

fn choose_swap_present_mode(
    avaliable_present_modes: &Vec<vk::PresentModeKHR>,
) -> vk::PresentModeKHR {
    for present_mode in avaliable_present_modes {
        if *present_mode == vk::PresentModeKHR::MAILBOX {
            return *present_mode;
        }
    }
    return vk::PresentModeKHR::FIFO;
}

fn choose_swap_extent(avaliable_capabilities: &vk::SurfaceCapabilitiesKHR) -> vk::Extent2D {
    if avaliable_capabilities.current_extent.width != std::u32::MAX {
        avaliable_capabilities.current_extent
    } else {
        use num::clamp;

        vk::Extent2D {
            width: clamp(
                WINDOW_WIDTH,
                avaliable_capabilities.min_image_extent.width,
                avaliable_capabilities.max_image_extent.width,
            ),
            height: clamp(
                WINDOW_HEIGHT,
                avaliable_capabilities.min_image_extent.height,
                avaliable_capabilities.max_image_extent.height,
            ),
        }
    }
}

fn create_swap_chain(
    instance: &ash::Instance,
    p_device: vk::PhysicalDevice,
    device: &ash::Device,
    surface_stuff: &SurfaceStuff,
    queue_family: &QueueFamilyIndices,
) -> SwapChainStuff {
    let detail = query_swap_chain_support(&instance, &surface_stuff, p_device);
    let surface_format = choose_swap_surface_format(&detail.formats);
    let present_mode = choose_swap_present_mode(&detail.present_modes);
    let swapchain_extent = choose_swap_extent(&detail.capabilities);

    let mut image_count = detail.capabilities.min_image_count + 1;
    if detail.capabilities.max_image_count > 0 && image_count > detail.capabilities.max_image_count
    {
        image_count = detail.capabilities.max_image_count;
    }

    let qf_indices = [
        queue_family.graphics_family.unwrap(),
        queue_family.present_family.unwrap(),
    ];
    let image_sharing_mode;
    let index_count;
    let indices_ptr;
    if qf_indices[0] != qf_indices[1] {
        image_sharing_mode = vk::SharingMode::CONCURRENT;
        index_count = 2u32;
        indices_ptr = qf_indices.as_ptr();
    } else {
        image_sharing_mode = vk::SharingMode::EXCLUSIVE;
        index_count = 0u32;
        indices_ptr = ptr::null();
    }

    let swapchain_ci = vk::SwapchainCreateInfoKHR {
        s_type: vk::StructureType::SWAPCHAIN_CREATE_INFO_KHR,
        p_next: ptr::null(),
        flags: vk::SwapchainCreateFlagsKHR::empty(),
        surface: surface_stuff.surface_khr,
        min_image_count: image_count,
        image_format: surface_format.format,
        image_color_space: surface_format.color_space,
        image_extent: swapchain_extent,
        image_array_layers: 1,
        image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
        image_sharing_mode: image_sharing_mode,
        queue_family_index_count: index_count,
        p_queue_family_indices: indices_ptr,
        pre_transform: detail.capabilities.current_transform,
        composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
        present_mode: present_mode,
        clipped: vk::TRUE,
        old_swapchain: vk::SwapchainKHR::null(),
    };

    let swapchain_loader = ash::extensions::khr::Swapchain::new(instance, device);
    let swapchain_khr = unsafe {
        swapchain_loader
            .create_swapchain(&swapchain_ci, None)
            .expect("Failed to create swapchain.")
    };
    let swapchain_image = unsafe {
        swapchain_loader
            .get_swapchain_images(swapchain_khr)
            .expect("Failed to get swapchain images.")
    };

    SwapChainStuff {
        swapchain_loader,
        swapchain_khr,
        swapchain_format: surface_format.format,
        swapchain_extent,
        swapchain_image,
    }
}

#[cfg(target_os = "windows")]
pub fn create_surface(
    entry: &ash::Entry,
    instance: &ash::Instance,
    window: &winit::window::Window,
) -> Result<vk::SurfaceKHR, vk::Result> {
    use std::os::raw::c_void;
    use std::ptr;
    use winapi::shared::windef::HWND;
    use winapi::um::libloaderapi::GetModuleHandleW;
    use winit::platform::windows::WindowExtWindows;

    let hwnd = window.hwnd() as HWND;
    let hinstance = unsafe { GetModuleHandleW(ptr::null()) as *const c_void };

    let win32_create_info = vk::Win32SurfaceCreateInfoKHR {
        s_type: vk::StructureType::WIN32_SURFACE_CREATE_INFO_KHR,
        p_next: ptr::null(),
        flags: Default::default(),
        hinstance,
        hwnd: hwnd as *const c_void,
    };
    let win32_surface_loader = Win32Surface::new(entry, instance);
    unsafe { win32_surface_loader.create_win32_surface(&win32_create_info, None) }
}

pub fn create_surface_stuff(
    entry: &ash::Entry,
    instance: &ash::Instance,
    window: &winit::window::Window,
) -> SurfaceStuff {
    let surface_khr = create_surface(entry, instance, window).expect("Failed to create surface.");

    let surface_loader = ash::extensions::khr::Surface::new(entry, instance);

    SurfaceStuff {
        surface_khr: surface_khr,
        surface_loader: surface_loader,
    }
}

fn create_render_pass(device: &ash::Device, swapchain_stuff: &SwapChainStuff) -> vk::RenderPass {
    let attachments = [vk::AttachmentDescription {
        flags: vk::AttachmentDescriptionFlags::empty(),
        format: swapchain_stuff.swapchain_format.clone(),
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
    }];

    let color_attachments_ref = [vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    }];

    let dependencies = [vk::SubpassDependency {
        src_subpass: vk::SUBPASS_EXTERNAL,
        dst_subpass: 0,
        src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        src_access_mask: vk::AccessFlags::empty(),
        dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
        dependency_flags: vk::DependencyFlags::empty(),
    }];

    let subpasses = [vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&color_attachments_ref)
        .build()];

    let render_pass_ci = vk::RenderPassCreateInfo::builder()
        .attachments(&attachments)
        .subpasses(&subpasses)
        .dependencies(&dependencies)
        .build();

    unsafe {
        device
            .create_render_pass(&render_pass_ci, None)
            .expect("Failed to create render pass.")
    }
}

fn create_image_views(
    device: &ash::Device,
    swapchain_stuff: &SwapChainStuff,
) -> Vec<vk::ImageView> {
    let mut image_views = Vec::with_capacity(swapchain_stuff.swapchain_image.len());
    for image in swapchain_stuff.swapchain_image.iter() {
        let image_view_ci = vk::ImageViewCreateInfo {
            s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::ImageViewCreateFlags::empty(),
            image: *image,
            view_type: vk::ImageViewType::TYPE_2D,
            format: swapchain_stuff.swapchain_format,
            components: vk::ComponentMapping {
                r: vk::ComponentSwizzle::IDENTITY,
                g: vk::ComponentSwizzle::IDENTITY,
                b: vk::ComponentSwizzle::IDENTITY,
                a: vk::ComponentSwizzle::IDENTITY,
            },
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
        };

        let image_view = unsafe {
            device
                .create_image_view(&image_view_ci, None)
                .expect("Failed to create image view.")
        };

        image_views.push(image_view);
    }

    image_views
}

fn create_graphics_pipeline(
    device: &ash::Device,
    swapchain_stuff: &SwapChainStuff,
    render_pass: vk::RenderPass,
) -> (vk::Pipeline, vk::PipelineLayout) {
    let vert_code = read_shader_code(std::path::Path::new("shader/spv/09_triangle.vert.spv"));
    let frag_code = read_shader_code(std::path::Path::new("shader/spv/09_triangle.frag.spv"));

    let vert_shader_module = create_shader_module(device, &vert_code);
    let frag_shader_module = create_shader_module(device, &frag_code);

    let main_function_name = CString::new("main").unwrap();

    let vert_pp_shader_stage_ci = vk::PipelineShaderStageCreateInfo {
        s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::PipelineShaderStageCreateFlags::empty(),
        stage: vk::ShaderStageFlags::VERTEX,
        module: vert_shader_module,
        p_name: main_function_name.as_ptr(),
        p_specialization_info: ptr::null(),
    };

    let frag_pp_shader_stage_ci = vk::PipelineShaderStageCreateInfo {
        s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::PipelineShaderStageCreateFlags::empty(),
        stage: vk::ShaderStageFlags::FRAGMENT,
        module: frag_shader_module,
        p_name: main_function_name.as_ptr(),
        p_specialization_info: ptr::null(),
    };

    let shader_stage_cis = [vert_pp_shader_stage_ci, frag_pp_shader_stage_ci];

    // vertex input state
    let vertex_input_ci = vk::PipelineVertexInputStateCreateInfo {
        s_type: vk::StructureType::PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::PipelineVertexInputStateCreateFlags::empty(),
        vertex_binding_description_count: 0,
        p_vertex_binding_descriptions: ptr::null(),
        vertex_attribute_description_count: 0,
        p_vertex_attribute_descriptions: ptr::null(),
    };

    // input assembly
    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo {
        s_type: vk::StructureType::PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::PipelineInputAssemblyStateCreateFlags::empty(),
        topology: vk::PrimitiveTopology::TRIANGLE_LIST,
        primitive_restart_enable: vk::FALSE,
    };

    // viewport
    let viewports = [vk::Viewport {
        x: 0f32,
        y: 0f32,
        width: swapchain_stuff.swapchain_extent.width as f32,
        height: swapchain_stuff.swapchain_extent.height as f32,
        min_depth: 0f32,
        max_depth: 1f32,
    }];

    // scissor
    let scissors = [vk::Rect2D {
        offset: vk::Offset2D { x: 0, y: 0 },
        extent: swapchain_stuff.swapchain_extent.clone(),
    }];

    let viewport_ci = vk::PipelineViewportStateCreateInfo {
        s_type: vk::StructureType::PIPELINE_VIEWPORT_STATE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::PipelineViewportStateCreateFlags::empty(),
        viewport_count: viewports.len() as u32,
        p_viewports: viewports.as_ptr(),
        scissor_count: scissors.len() as u32,
        p_scissors: scissors.as_ptr(),
    };

    // rasterizer
    let rasterization_ci = vk::PipelineRasterizationStateCreateInfo {
        s_type: vk::StructureType::PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::PipelineRasterizationStateCreateFlags::empty(),
        depth_clamp_enable: vk::FALSE,
        rasterizer_discard_enable: vk::FALSE,
        polygon_mode: vk::PolygonMode::FILL,
        cull_mode: vk::CullModeFlags::BACK,
        front_face: vk::FrontFace::CLOCKWISE,
        depth_bias_enable: vk::FALSE,
        depth_bias_constant_factor: 0f32,
        depth_bias_clamp: 0f32,
        depth_bias_slope_factor: 0f32,
        line_width: 1f32,
    };

    // multisample
    let multisample_ci = vk::PipelineMultisampleStateCreateInfo {
        s_type: vk::StructureType::PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::PipelineMultisampleStateCreateFlags::empty(),
        rasterization_samples: vk::SampleCountFlags::TYPE_1,
        sample_shading_enable: vk::FALSE,
        min_sample_shading: 1f32,
        p_sample_mask: ptr::null(),
        alpha_to_coverage_enable: vk::FALSE,
        alpha_to_one_enable: vk::FALSE,
    };

    let stencil_state = vk::StencilOpState {
        fail_op: vk::StencilOp::KEEP,
        pass_op: vk::StencilOp::KEEP,
        depth_fail_op: vk::StencilOp::KEEP,
        compare_op: vk::CompareOp::ALWAYS,
        compare_mask: 0,
        write_mask: 0,
        reference: 0,
    };

    let depth_stencil_ci = vk::PipelineDepthStencilStateCreateInfo {
        s_type: vk::StructureType::PIPELINE_DEPTH_STENCIL_STATE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::PipelineDepthStencilStateCreateFlags::empty(),
        depth_test_enable: vk::FALSE,
        depth_write_enable: vk::FALSE,
        depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,
        depth_bounds_test_enable: vk::FALSE,
        stencil_test_enable: vk::FALSE,
        front: stencil_state,
        back: stencil_state,
        max_depth_bounds: 1.0,
        min_depth_bounds: 0.0,
    };

    let color_blend_attachment_state = [vk::PipelineColorBlendAttachmentState {
        color_write_mask: vk::ColorComponentFlags::all(),
        blend_enable: vk::FALSE,
        src_color_blend_factor: vk::BlendFactor::ONE,
        dst_color_blend_factor: vk::BlendFactor::ZERO,
        color_blend_op: vk::BlendOp::ADD,
        src_alpha_blend_factor: vk::BlendFactor::ONE,
        dst_alpha_blend_factor: vk::BlendFactor::ZERO,
        alpha_blend_op: vk::BlendOp::ADD,
    }];

    let color_blend_ci = vk::PipelineColorBlendStateCreateInfo {
        s_type: vk::StructureType::PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::PipelineColorBlendStateCreateFlags::empty(),
        logic_op_enable: vk::FALSE,
        logic_op: vk::LogicOp::COPY,
        attachment_count: color_blend_attachment_state.len() as u32,
        p_attachments: color_blend_attachment_state.as_ptr(),
        blend_constants: [0f32; 4],
    };

    let dynamic_state = [vk::DynamicState::VIEWPORT, vk::DynamicState::LINE_WIDTH];

    let dynamic_ci = vk::PipelineDynamicStateCreateInfo {
        s_type: vk::StructureType::PIPELINE_DYNAMIC_STATE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::PipelineDynamicStateCreateFlags::empty(),
        dynamic_state_count: dynamic_state.len() as u32,
        p_dynamic_states: dynamic_state.as_ptr(),
    };

    // pipeline layout create info
    let pp_layout_ci = vk::PipelineLayoutCreateInfo {
        s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::PipelineLayoutCreateFlags::empty(),
        set_layout_count: 0,
        p_set_layouts: ptr::null(),
        push_constant_range_count: 0,
        p_push_constant_ranges: ptr::null(),
    };

    let pp_layout = unsafe {
        device
            .create_pipeline_layout(&pp_layout_ci, None)
            .expect("Failed create pipeline layout.")
    };

    let pipeline_ci = vk::GraphicsPipelineCreateInfo::builder()
        .stages(&shader_stage_cis)
        .vertex_input_state(&vertex_input_ci)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_ci)
        .rasterization_state(&rasterization_ci)
        .multisample_state(&multisample_ci)
        .depth_stencil_state(&depth_stencil_ci)
        .color_blend_state(&color_blend_ci)
        .dynamic_state(&dynamic_ci)
        .layout(pp_layout)
        .render_pass(render_pass)
        .build();

    let graphics_pipelines = unsafe {
        device
            .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_ci], None)
            .expect("Failed to create graphics pipeline")
    };

    unsafe {
        device.destroy_shader_module(vert_shader_module, None);
        device.destroy_shader_module(frag_shader_module, None);
    };

    (graphics_pipelines[0], pp_layout)
}

fn read_shader_code(shader_path: &std::path::Path) -> Vec<u8> {
    use std::fs::File;
    use std::io::Read;

    let spv_file =
        File::open(shader_path).expect(&format!("Failed to open file at {:?}", shader_path));
    let bytes_code: Vec<u8> = spv_file.bytes().filter_map(|byte| byte.ok()).collect();
    bytes_code
}

fn create_shader_module(device: &ash::Device, shader_code: &Vec<u8>) -> vk::ShaderModule {
    let shader_module_ci = vk::ShaderModuleCreateInfo {
        s_type: vk::StructureType::SHADER_MODULE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::ShaderModuleCreateFlags::empty(),
        code_size: shader_code.len(),
        p_code: shader_code.as_ptr() as *const u32,
    };

    unsafe {
        device
            .create_shader_module(&shader_module_ci, None)
            .expect("Failed to create shader modules.")
    }
}

fn create_framebuffer(
    device: &ash::Device,
    swapchain_stuff: &SwapChainStuff,
    swapchain_image_views: &Vec<vk::ImageView>,
    render_pass: vk::RenderPass,
) -> Vec<vk::Framebuffer> {
    let mut framebuffers = Vec::new();
    for &image_view in swapchain_image_views.iter() {
        let attachments = [image_view];

        let framebuffer_ci = vk::FramebufferCreateInfo {
            s_type: vk::StructureType::FRAMEBUFFER_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::FramebufferCreateFlags::empty(),
            attachment_count: attachments.len() as u32,
            p_attachments: attachments.as_ptr(),
            width: swapchain_stuff.swapchain_extent.width,
            height: swapchain_stuff.swapchain_extent.height,
            render_pass: render_pass,
            layers: 1,
        };

        let framebuffer = unsafe {
            device
                .create_framebuffer(&framebuffer_ci, None)
                .expect("Failed to create framebuffer.")
        };

        framebuffers.push(framebuffer);
    }

    framebuffers
}

fn create_command_pool(
    device: &ash::Device,
    queue_family_indices: &QueueFamilyIndices,
) -> vk::CommandPool {
    let command_pool_ci = vk::CommandPoolCreateInfo {
        s_type: vk::StructureType::COMMAND_POOL_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::CommandPoolCreateFlags::empty(),
        queue_family_index: queue_family_indices.graphics_family.unwrap(),
    };

    unsafe {
        device
            .create_command_pool(&command_pool_ci, None)
            .expect("Failed to create command pool.")
    }
}

fn create_command_buffers(
    device: &ash::Device,
    swapchain_stuff: &SwapChainStuff,
    command_pool: vk::CommandPool,
    render_pass: vk::RenderPass,
    framebuffers: &Vec<vk::Framebuffer>,
    pipeline: vk::Pipeline,
) -> Vec<vk::CommandBuffer> {
    let command_buffer_ai = vk::CommandBufferAllocateInfo {
        s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
        p_next: ptr::null(),
        command_pool: command_pool,
        level: vk::CommandBufferLevel::PRIMARY,
        command_buffer_count: swapchain_stuff.swapchain_image.len() as u32,
    };

    let command_buffers = unsafe {
        device
            .allocate_command_buffers(&command_buffer_ai)
            .expect("Failed to allocate command buffers.")
    };

    for (idx, &cmd) in command_buffers.iter().enumerate() {
        let cmd_begin_info = vk::CommandBufferBeginInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
            p_next: ptr::null(),
            flags: vk::CommandBufferUsageFlags::SIMULTANEOUS_USE,
            p_inheritance_info: ptr::null(),
        };

        unsafe {
            device
                .begin_command_buffer(cmd, &cmd_begin_info)
                .expect("Failed to begin command buffer.");
        }

        let clear_value = [vk::ClearValue {
            color: vk::ClearColorValue { float32: [0f32; 4] },
        }];

        let render_pass_info = vk::RenderPassBeginInfo {
            s_type: vk::StructureType::RENDER_PASS_BEGIN_INFO,
            p_next: ptr::null(),
            render_pass: render_pass,
            framebuffer: framebuffers[idx],
            render_area: vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: swapchain_stuff.swapchain_extent,
            },
            clear_value_count: clear_value.len() as u32,
            p_clear_values: clear_value.as_ptr(),
        };

        let viewports = [vk::Viewport {
            x: 0f32,
            y: 0f32,
            width: swapchain_stuff.swapchain_extent.width as f32,
            height: swapchain_stuff.swapchain_extent.height as f32,
            min_depth: 0f32,
            max_depth: 1f32,
        }];

        unsafe {
            // render pass
            device.cmd_begin_render_pass(cmd, &render_pass_info, vk::SubpassContents::INLINE);
            // pipeline
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipeline);
            // viewport
            device.cmd_set_viewport(cmd, 0, &viewports);
            // draw
            device.cmd_draw(cmd, 3, 1, 0, 0);
            // end render pass
            device.cmd_end_render_pass(cmd);
            // end command buffer
            device
                .end_command_buffer(cmd)
                .expect("Failed to end command buffer.");
        }
    }

    command_buffers
}

fn create_semaphore(device: &ash::Device) -> (vk::Semaphore, vk::Semaphore) {
    let semaphor_ci = vk::SemaphoreCreateInfo::builder().build();
    let image_avaliable_semaphore = unsafe {
        device
            .create_semaphore(&semaphor_ci, None)
            .expect("Failed to create semaphore.")
    };
    let render_finished_semaphore = unsafe {
        device
            .create_semaphore(&semaphor_ci, None)
            .expect("Failed to create semaphore.")
    };

    (image_avaliable_semaphore, render_finished_semaphore)
}

pub struct SurfaceStuff {
    surface_loader: ash::extensions::khr::Surface,
    surface_khr: vk::SurfaceKHR,
}

struct App {
    entry: ash::Entry,
    instance: ash::Instance,
    surface_loader: ash::extensions::khr::Surface,
    surface_khr: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
    device: ash::Device, // logic device
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
    // swapchain
    swapchain_loader: ash::extensions::khr::Swapchain,
    swapchain_khr: vk::SwapchainKHR,
    swapchain_image: Vec<vk::Image>,
    swapchain_format: vk::Format,
    swapchain_extent: vk::Extent2D,
    swapchain_image_views: Vec<vk::ImageView>,
    //
    pipeline_layout: vk::PipelineLayout,
    graphic_pipeline: vk::Pipeline,
    render_pass: vk::RenderPass,
    swapchain_framebuffers: Vec<vk::Framebuffer>,
    //
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,

    image_avaliable_semaphore: vk::Semaphore,
    render_finished_semaphore: vk::Semaphore,

    debug_utils_loader: ash::extensions::ext::DebugUtils,
    debug_utils_messenger: vk::DebugUtilsMessengerEXT,
}

const VALIDATION_INFO: ValidationInfo = ValidationInfo {
    enable_validation: true,
    required_validation_layers: ["VK_LAYER_KHRONOS_validation"],
};

const DEVICE_EXTENSIONS: DeviceExtension = DeviceExtension {
    name: ["VK_KHR_swapchain"],
};

impl App {
    pub fn new(window: &winit::window::Window) -> App {
        let entry = unsafe { ash::Entry::new().unwrap() };

        if VALIDATION_INFO.enable_validation
            && !check_validation_layer_support(&entry, &VALIDATION_INFO.required_validation_layers)
        {
            panic!("validation layers requested, but not avaliable!");
        }

        let debug_utils_messenger_ci = get_debug_utils_messenger_create_info();
        let instance = App::create_vk_instance(&entry, &debug_utils_messenger_ci);

        let debug_utils_loader = ash::extensions::ext::DebugUtils::new(&entry, &instance);
        let debug_utils_messenger =
            get_debug_messenger(&debug_utils_messenger_ci, &debug_utils_loader);

        let surface_stuff = create_surface_stuff(&entry, &instance, window);

        let physical_device = pick_physic_device(&instance, &surface_stuff);

        let queue_family_indices = find_queue_family(&instance, physical_device, &surface_stuff);

        let logical_device = create_logic_device(&instance, physical_device, &queue_family_indices);

        let graphics_queue = unsafe {
            logical_device.get_device_queue(queue_family_indices.graphics_family.unwrap(), 0)
        };

        let present_queue = unsafe {
            logical_device.get_device_queue(queue_family_indices.present_family.unwrap(), 0)
        };

        let swapchain_stuff = create_swap_chain(
            &instance,
            physical_device,
            &logical_device,
            &surface_stuff,
            &queue_family_indices,
        );

        let swapchain_image_views = create_image_views(&logical_device, &swapchain_stuff);

        let render_pass = create_render_pass(&logical_device, &swapchain_stuff);

        let (pipeline, pipeline_layout) =
            create_graphics_pipeline(&logical_device, &swapchain_stuff, render_pass);

        let framebuffers = create_framebuffer(
            &logical_device,
            &swapchain_stuff,
            &swapchain_image_views,
            render_pass,
        );

        let command_pool = create_command_pool(&logical_device, &queue_family_indices);

        let command_buffers = create_command_buffers(
            &logical_device,
            &swapchain_stuff,
            command_pool,
            render_pass,
            &framebuffers,
            pipeline,
        );

        let (image_avaliable_semaphore, render_finished_semaphore) =
            create_semaphore(&logical_device);

        App {
            entry: entry,
            instance: instance,
            surface_loader: surface_stuff.surface_loader,
            surface_khr: surface_stuff.surface_khr,
            physical_device: physical_device,
            device: logical_device,
            graphics_queue: graphics_queue,
            present_queue: present_queue,
            // swapchain
            swapchain_loader: swapchain_stuff.swapchain_loader,
            swapchain_khr: swapchain_stuff.swapchain_khr,
            swapchain_image: swapchain_stuff.swapchain_image,
            swapchain_format: swapchain_stuff.swapchain_format,
            swapchain_extent: swapchain_stuff.swapchain_extent,
            swapchain_image_views: swapchain_image_views,
            //
            pipeline_layout: pipeline_layout,
            graphic_pipeline: pipeline,
            render_pass: render_pass,
            swapchain_framebuffers: framebuffers,
            //
            command_pool: command_pool,
            command_buffers: command_buffers,
            image_avaliable_semaphore: image_avaliable_semaphore,
            render_finished_semaphore: render_finished_semaphore,

            debug_utils_loader: debug_utils_loader,
            debug_utils_messenger: debug_utils_messenger,
        }
    }

    fn create_vk_instance(
        entry: &ash::Entry,
        debug_utils_messenger_ci: &vk::DebugUtilsMessengerCreateInfoEXT,
    ) -> ash::Instance {
        let app_name = CString::new(WINDOW_TITLE).unwrap();
        let engine_name = CString::new("Vulkan").unwrap();

        let app_info = vk::ApplicationInfo {
            s_type: vk::StructureType::APPLICATION_INFO,
            p_next: ptr::null(),
            p_application_name: app_name.as_ptr(),
            application_version: APPLICATION_VERSION,
            p_engine_name: engine_name.as_ptr(),
            engine_version: ENGINE_VERSION,
            api_version: vk::API_VERSION_1_0,
        };

        let require_validataion_layer_raw_names = get_require_layer_raw_names();

        let extension_names = required_extension_names();

        let instance_create_info = vk::InstanceCreateInfo {
            s_type: vk::StructureType::INSTANCE_CREATE_INFO,
            p_next: if VALIDATION_INFO.enable_validation {
                debug_utils_messenger_ci as *const vk::DebugUtilsMessengerCreateInfoEXT
                    as *const c_void
            } else {
                ptr::null()
            },
            flags: vk::InstanceCreateFlags::default(),
            p_application_info: &app_info,
            pp_enabled_layer_names: require_validataion_layer_raw_names.as_ptr(),
            enabled_layer_count: require_validataion_layer_raw_names.len() as u32,
            pp_enabled_extension_names: extension_names.as_ptr(),
            enabled_extension_count: extension_names.len() as u32,
        };

        unsafe {
            entry
                .create_instance(&instance_create_info, None)
                .expect("Failed to create instance")
        }
    }

    fn init_window(event_loop: &EventLoop<()>) -> winit::window::Window {
        winit::window::WindowBuilder::new()
            .with_title(WINDOW_TITLE)
            .with_inner_size(winit::dpi::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
            .build(event_loop)
            .expect("Failed to create window.")
    }

    pub fn main_loop(mut self, event_loop: EventLoop<()>, window: Window) {
        event_loop.run(move |event, _, control_flow| match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::KeyboardInput { input, .. } => match input {
                    KeyboardInput {
                        virtual_keycode,
                        state,
                        ..
                    } => match (virtual_keycode, state) {
                        (Some(VirtualKeyCode::Escape), ElementState::Pressed) => {
                            dbg!("??????Esc");
                            *control_flow = ControlFlow::Exit;
                        }
                        _ => (),
                    },
                },
                _ => (),
            },
            Event::MainEventsCleared => window.request_redraw(),
            Event::RedrawRequested(_window_id) => {
                self.draw_frame();
            }
            _ => (),
        })
    }

    pub fn draw_frame(&mut self) {
        // println!("draw")
        let (image_idx, _) = unsafe {
            self.swapchain_loader
                .acquire_next_image(
                    self.swapchain_khr,
                    u64::MAX,
                    self.image_avaliable_semaphore,
                    vk::Fence::null(),
                )
                .expect("Failed to acquire next image.")
        };

        let wait_semaphores = [self.image_avaliable_semaphore];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = [self.render_finished_semaphore];

        let submit_info = vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            p_next: ptr::null(),
            wait_semaphore_count: wait_semaphores.len() as u32,
            p_wait_semaphores: wait_semaphores.as_ptr(),
            p_wait_dst_stage_mask: wait_stages.as_ptr(),
            command_buffer_count: 1,
            p_command_buffers: &self.command_buffers[image_idx as usize],
            signal_semaphore_count: signal_semaphores.len() as u32,
            p_signal_semaphores: signal_semaphores.as_ptr(),
        };

        let swapchains = [self.swapchain_khr];

        let present_info = vk::PresentInfoKHR {
            s_type: vk::StructureType::PRESENT_INFO_KHR,
            p_next: ptr::null(),
            wait_semaphore_count: 1,
            p_wait_semaphores: &self.render_finished_semaphore,
            swapchain_count: swapchains.len() as u32,
            p_swapchains: swapchains.as_ptr(),
            p_image_indices: &image_idx,
            p_results: ptr::null_mut(),
        };

        // submit to graphics queue
        unsafe {
            self.device
                .queue_submit(self.graphics_queue, &[submit_info], vk::Fence::null())
                .expect("Failed to queue submit.");
            self.swapchain_loader
                .queue_present(self.present_queue, &present_info)
                .expect("Failed to queue present.");
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        unsafe {
            // self.device.queue_wait_idle(self.graphics_queue)
            //     .expect("Failed to wait graphics queue idle");
            // self.device.queue_wait_idle(self.present_queue)
            //     .expect("Failed to wait present queue idle");
            self.device
                .device_wait_idle()
                .expect("Failed to wait device idle");
            self.device
                .destroy_semaphore(self.image_avaliable_semaphore, None);
            self.device
                .destroy_semaphore(self.render_finished_semaphore, None);
            self.device.destroy_command_pool(self.command_pool, None);
            for framebuffer in self.swapchain_framebuffers.iter() {
                self.device.destroy_framebuffer(*framebuffer, None);
            }
            self.device.destroy_pipeline(self.graphic_pipeline, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_render_pass(self.render_pass, None);

            for &image_view in self.swapchain_image_views.iter() {
                self.device.destroy_image_view(image_view, None);
            }
            self.swapchain_loader
                .destroy_swapchain(self.swapchain_khr, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface_khr, None);
            if VALIDATION_INFO.enable_validation {
                self.debug_utils_loader
                    .destroy_debug_utils_messenger(self.debug_utils_messenger, None);
            }
            self.instance.destroy_instance(None);
        }
    }
}

fn main() {
    let event_loop = EventLoop::new();
    let _window = App::init_window(&event_loop);
    let app = App::new(&_window);

    app.main_loop(event_loop, _window);
}
