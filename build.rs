fn main() {
    // Manifest erzwingt Administrator-Rechte (EnergyDrv braucht sie)
    embed_resource::compile("app.rc", embed_resource::NONE)
        .manifest_required()
        .unwrap();
}
