use winit::event::{Event, VirtualKeyCode, ElementState,KeyboardInput, WindowEvent};
use winit::event_loop::{EventLoop, ControlFlow};
use winit::window::Window;

use ash::{vk, version::EntryV1_0, version::InstanceV1_0};
use std::ffi::{CStr, CString, c_void};
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
    p_use_data: *mut c_void
) -> vk::Bool32{

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

    println!("[Debug]{}{}{:?}", message_severity_str, message_type_str, message);

    vk::FALSE
}

pub fn check_validation_layer_support(
    entry: &ash::Entry,
    layers: &[&'static str]
) -> bool {
    let layer_properties = entry.enumerate_instance_layer_properties()
        .expect("Failed to enumerate Instance Layers Properties");

    for check_layer in layers.iter() {
        let mut found = false;
        for property in layer_properties.iter() {
            let c_str = unsafe { 
                let ptr = property.layer_name.as_ptr();
                CStr::from_ptr(ptr)
            }.to_str()
            .expect("Failed to convert vulkan raw pointer");

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

fn get_debug_utils_messenger_create_info() 
-> vk::DebugUtilsMessengerCreateInfoEXT {        
    vk::DebugUtilsMessengerCreateInfoEXT {
        s_type: vk::StructureType::DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
        p_next: ptr::null(),
        flags: vk::DebugUtilsMessengerCreateFlagsEXT::empty(),
        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE | 
            vk::DebugUtilsMessageSeverityFlagsEXT::WARNING |
            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
        message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL |
            vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION |
            vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
        pfn_user_callback: Some(vulkan_debug_utils_debug),
        p_user_data: ptr::null_mut(),
    }
}

fn get_debug_messenger(create_info: &vk::DebugUtilsMessengerCreateInfoEXT, debug_utils_loader: &ash::extensions::ext::DebugUtils) 
-> vk::DebugUtilsMessengerEXT {
    if !VALIDATION_INFO.enable_validation {
        vk::DebugUtilsMessengerEXT::null()
    }
    else {
        let utils_messenger = unsafe { 
            debug_utils_loader
                .create_debug_utils_messenger(&create_info, None)
                .expect("Failed to set up debug messenger!")
        };

        utils_messenger
    }
}
pub struct ValidationInfo {
    pub enable_validation: bool,
    pub required_validation_layers: [&'static str; 1],
}


struct App {
    entry: ash::Entry,
    instance: ash::Instance,
    debug_utils_loader: ash::extensions::ext::DebugUtils,
    debug_utils_messenger: vk::DebugUtilsMessengerEXT,
}


const VALIDATION_INFO: ValidationInfo = ValidationInfo{
    enable_validation: true,
    required_validation_layers: ["VK_LAYER_KHRONOS_validation"]
};


impl App {
    pub fn new() -> App {
        let entry = unsafe { ash::Entry::new().unwrap() };

        if VALIDATION_INFO.enable_validation && !check_validation_layer_support(&entry, &VALIDATION_INFO.required_validation_layers) {
            panic!("validation layers requested, but not avaliable!");
        }

        let debug_utils_messenger_ci = get_debug_utils_messenger_create_info();
        let instance = App::create_vk_instance(&entry, &debug_utils_messenger_ci);

        let debug_utils_loader = ash::extensions::ext::DebugUtils::new(&entry, &instance);
        let debug_utils_messenger = get_debug_messenger(&debug_utils_messenger_ci, &debug_utils_loader);

        App {
            entry: entry,
            instance: instance,
            debug_utils_loader: debug_utils_loader,
            debug_utils_messenger: debug_utils_messenger,
        }
    }

    fn create_vk_instance(entry: &ash::Entry, debug_utils_messenger_ci: &vk::DebugUtilsMessengerCreateInfoEXT) 
    -> ash::Instance{
        let app_name = CString::new(WINDOW_TITLE).unwrap();
        let engine_name = CString::new("Vulkan").unwrap();

        let app_info = vk::ApplicationInfo {
            s_type: vk::StructureType::APPLICATION_INFO,
            p_next: ptr::null(),
            p_application_name: app_name.as_ptr(),
            application_version: APPLICATION_VERSION,
            p_engine_name: engine_name.as_ptr(),
            engine_version: ENGINE_VERSION,
            api_version: vk::API_VERSION_1_0
        };

        let require_validataion_layer_raw_names = VALIDATION_INFO
            .required_validation_layers
            .iter()
            .map(|layer_name| *layer_name as *const str as *const i8 )
            .collect::<Vec<*const i8>>();
        
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
            pp_enabled_layer_names: if VALIDATION_INFO.enable_validation {
                require_validataion_layer_raw_names.as_ptr()
            } else {
                ptr::null()
            },
            enabled_layer_count: if VALIDATION_INFO.enable_validation {
                require_validataion_layer_raw_names.len() as u32
            } else {
                0u32
            },
            pp_enabled_extension_names: extension_names.as_ptr(),
            enabled_extension_count: extension_names.len() as u32 , 
        };

        unsafe {
            entry.create_instance(&instance_create_info, None)
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

    pub fn init_vulkan() {
        
    }

    pub fn main_loop(mut self, event_loop: EventLoop<()>, window: Window){
        event_loop.run(move |event, _, control_flow| {
            match event {
                Event::WindowEvent { event, ..} => {
                    match event {
                        WindowEvent::CloseRequested => {
                            *control_flow = ControlFlow::Exit;
                        },
                        WindowEvent::KeyboardInput { input, ..} => {
                            match input {
                                KeyboardInput { virtual_keycode, state, .. } => {
                                    match (virtual_keycode, state) {
                                        (Some(VirtualKeyCode::Escape), ElementState::Pressed) => {
                                            dbg!("按下Esc");
                                            *control_flow = ControlFlow::Exit;
                                        },
                                        _ => ()
                                    }
                                }
                            }
                        },
                        _ => ()
                    }
                },
                Event::MainEventsCleared => {
                    window.request_redraw()
                },
                Event::RedrawRequested(_window_id) => {
                    self.draw_frame();
                },
                _ => (),
            }
        })
    }

    pub fn draw_frame(&mut self) {
        // println!("draw")
    }

}

impl Drop for App {
    fn drop(&mut self) {
        if VALIDATION_INFO.enable_validation {
            unsafe{
                self.debug_utils_loader.destroy_debug_utils_messenger(self.debug_utils_messenger, None);
            }
        }
        unsafe {
            self.instance.destroy_instance(None);
        }

    }
}



fn main() {
    let event_loop = EventLoop::new();
    let app = App::new();
    let _window = App::init_window(&event_loop);

    app.main_loop(event_loop, _window);
}
