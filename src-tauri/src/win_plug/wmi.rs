use anyhow::{anyhow, Context};
use serde::Deserialize;
use windows::core::{w, BSTR, VARIANT};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoInitializeSecurity, CLSCTX_INPROC_SERVER,
    COINIT_MULTITHREADED, EOAC_NONE, RPC_C_AUTHN_LEVEL_DEFAULT, RPC_C_IMP_LEVEL_IMPERSONATE,
};
use windows::Win32::System::Wmi::{
    IWbemClassObject, IWbemLocator, IWbemServices, WbemLocator, WBEM_FLAG_FORWARD_ONLY,
    WBEM_FLAG_RETURN_ERROR_OBJECT, WBEM_FLAG_RETURN_WBEM_COMPLETE, WBEM_INFINITE,
};
use wmi::{COMLibrary, WMIConnection};
use colored::Colorize;

pub fn wmi_security() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let _ = CoInitializeSecurity(
            None,
            -1,
            None,
            None,
            RPC_C_AUTHN_LEVEL_DEFAULT,
            RPC_C_IMP_LEVEL_IMPERSONATE,
            None,
            EOAC_NONE,
            None,
        );
    }
}

pub fn wmi_init() -> (IWbemClassObject, IWbemServices, BSTR, BSTR) {
    // Connect to the required namespace on the local DCOM server.
    let loc: IWbemLocator = unsafe {
        CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER)
            .context("Failed to get WbemLocator")
            .unwrap()
    };
    let svc = unsafe {
        loc.ConnectServer(&BSTR::from(r"ROOT\WMI"), None, None, None, 0, None, None)
            .context("Connecting to server")
            .expect("Connecting Error")
    };

    // Allocate null-terminated 16-bit character strings for the object class name and method name.
    let cls_name = BSTR::from("AcpiTest_MULong");
    let method_name = BSTR::from("GetSetULong");

    // List instances of the requested object by name, and get the path of the first.
    let object_enum = unsafe {
        svc.CreateInstanceEnum(
            &cls_name,
            WBEM_FLAG_FORWARD_ONLY | WBEM_FLAG_RETURN_ERROR_OBJECT,
            None,
        )
        .context("Get cls_name")
        .expect("Get cls_name Error")
    };
    let mut objects = [None; 1];
    let mut count: u32 = 0;
    unsafe {
        object_enum
            .Next(WBEM_INFINITE, &mut objects, &mut count)
            .ok()
            .context("Retrieving first")
            .unwrap();
    }
    let mut obj_path = VARIANT::new();
    unsafe {
        objects[0]
            .as_ref()
            .ok_or_else(|| anyhow!("Missing object"))
            .expect("Error")
            .Get(w!("__RELPATH"), 0, &mut obj_path, None, None)
            .context("Retrieving object path")
            .expect("Error");
    }
    let obj_path = BSTR::try_from(&obj_path)
        .context("Converting object path to string")
        .expect("Error");
    drop(objects);
    drop(object_enum);
    // println!("Instance: {obj_path}");

    // Get an input parameter object from the object class.
    let mut cls: Option<IWbemClassObject> = None;
    unsafe {
        svc.GetObject(
            &cls_name,
            WBEM_FLAG_RETURN_WBEM_COMPLETE,
            None,
            Some(&mut cls),
            None,
        )
        .context("Getting class MSFT_Disk")
        .unwrap();
    }
    let cls = cls.ok_or_else(|| anyhow!("Missing class")).expect("Error");
    let mut in_cls: Option<IWbemClassObject> = None;
    let mut out_cls: Option<IWbemClassObject> = None;
    unsafe {
        cls.GetMethod(&method_name, 0, &mut in_cls, &mut out_cls)
            .context("Getting method")
            .expect("Get method Error");
    }
    // println!("{}", "初始化 WMI".green());
    (in_cls.unwrap(), svc, obj_path, method_name)
}

pub fn wmi_set(in_cls: &IWbemClassObject, svc: &IWbemServices, obj_path: &BSTR, method_name: &BSTR, size: &str) -> i64 {
    let in_params = match unsafe { in_cls.SpawnInstance(0) } {
        Ok(params) => params,
        Err(e) => {
            println!("创建输入参数失败: {:?}", e);
            return -1;
        }
    };

    let data_value = match u64::from_str_radix(&size[2..], 16) {
        Ok(val) => val,
        Err(e) => {
            println!("解析十六进制值失败 '{}': {:?}", size, e);
            return -1;
        }
    };

    unsafe {
        if let Err(e) = in_params.Put(
            &BSTR::from("Data"),
            0,
            &VARIANT::from(data_value.to_string().as_str()),
            0,
        ) {
            println!("设置Data参数失败: {:?}", e);
            return -1;
        }
    }

    let mut out_params: Option<IWbemClassObject> = None;
    if let Err(e) = unsafe {
        svc.ExecMethod(
            obj_path,
            method_name,
            WBEM_FLAG_RETURN_WBEM_COMPLETE,
            None,
            &in_params,
            Some(&mut out_params),
            None,
        )
    } {
        println!("执行WMI方法失败: {:?}", e);
        return -1;
    }

    let out_params = match out_params {
        Some(params) => params,
        None => {
            println!("未收到输出参数");
            return -1;
        }
    };

    let mut return_value = VARIANT::new();
    if let Err(e) = unsafe {
        out_params.Get(w!("Return"), 0, &mut return_value, None, None)
    } {
        println!("获取返回值失败: {:?}", e);
        return -1;
    }

    // 修复问题13：解析失败时打印原始值和错误，而非静默 unwrap_or(-1)
    let raw = return_value.to_string();
    match raw.parse::<i64>() {
        Ok(v) => v,
        Err(e) => {
            println!("WMI 返回值解析失败 原始值='{}': {:?}", raw, e);
            -1
        }
    }
}

pub fn get_model() -> i64 {
    #[derive(Deserialize, Debug)]
    struct LaptopInfo {
        model: String,
        manufacturer: String,
    }
    let com_con = COMLibrary::new().unwrap();
    let wmi_con = WMIConnection::new(com_con.into()).unwrap();
    let results: Vec<LaptopInfo> = wmi_con
        .raw_query("SELECT Model, Manufacturer FROM Win32_ComputerSystem")
        .unwrap();
    for laptop in results {
        println!("Manufacturer: {}", laptop.manufacturer.blue());
        println!("Model: {}", laptop.model.blue());
        match laptop.model.as_str() {
            "LAPKC71F" => return 0,
            "LAPAC71H" => return 1,
            _ => return 0,
        }
    }
    0
}
