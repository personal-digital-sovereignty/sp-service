use std::process::Command;

fn main() {
    // Notify Cargo to re-run this build script only if the python script changes
    // (though actually we want it to run on every build to check the hash dynamically, 
    // so we don't set a rerun-if-changed, or we rely on the python script to cash the hash).
    // To ensure it runs every time we build:
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../scripts/fetch_public_apis.py");
    
    println!("cargo:warning=O Sovereign Build System está vetorizando a lista de APIs públicas...");
    
    let status = Command::new("python3")
        .arg("../scripts/fetch_public_apis.py")
        .status();
        
    match status {
        Ok(s) if s.success() => {
            println!("cargo:warning=APIs listadas com sucesso (Base64 gerado).");
        }
        _ => {
            println!("cargo:warning=Falha ao executar o Python scraper (Fallback vazio será ativado).");
        }
    }
}
