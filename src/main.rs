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

    println!(
        "[Debug]{}{}{:?}",
        message_severity_str, message_type_str, message
    );

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

    // 输出gpu设备信息
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
    // 选择设备
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

fn create_image_views(device: &ash::Device, swapchain_stuff: &SwapChainStuff) -> Vec<ImageView> {
    let image_views = Vec::with_capacity(swapchain_stuff.swapchain_image.len());
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
                            dbg!("按下Esc");
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
    }
}

impl Drop for App {
    fn drop(&mut self) {
        unsafe {
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
