fn main() {
    if cfg!(target_os = "windows") {
        // 编译资源文件
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/favicon.ico");
        res.compile().unwrap();
        
        // 编译C++代码
        cc::Build::new()
            .cpp(true)
            .std("c++17")
            .file("cpp-tsf/src/tsf_service.cpp")
            .include("cpp-tsf/include")
            .define("UNICODE", None)
            .define("_UNICODE", None)
            .compile("tsf_service");
        
        // 链接必要的Windows库
        println!("cargo:rustc-link-lib=ole32");
        println!("cargo:rustc-link-lib=oleaut32");
        println!("cargo:rustc-link-lib=uuid");
        println!("cargo:rustc-link-lib=user32");
        println!("cargo:rustc-link-lib=advapi32");
        println!("cargo:rustc-link-lib=kernel32");
        
        // 重新构建条件
        println!("cargo:rerun-if-changed=cpp-tsf/src/tsf_service.cpp");
        println!("cargo:rerun-if-changed=cpp-tsf/include/tsf_service.h");
        println!("cargo:rerun-if-changed=build.rs");
        println!("cargo:rerun-if-changed=assets/favicon.ico");
        println!("cargo:rerun-if-changed=app.rc");
        println!("cargo:rerun-if-changed=assets/彝文音节字典.json");
        println!("cargo:rerun-if-changed=assets/彝文部首字典.json");
    }
}
