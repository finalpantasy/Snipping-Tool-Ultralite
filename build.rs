fn main() {
    println!("cargo:rerun-if-changed=assets/ul-icon.ico");
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        // Preserve every authored size instead of synthesizing a single 256px entry.
        res.set_icon("assets/ul-icon.ico");
        res.set("ProductName", "Snipping Tool UL");
        res.set("CompanyName", "Snipping Tool UL contributors");
        res.set("FileDescription", "Snipping Tool UL");
        res.set(
            "LegalCopyright",
            "Copyright 2026 Snipping Tool UL contributors; MIT third-party notices included",
        );
        res.compile().expect("winres resource compile failed");
    }
}
