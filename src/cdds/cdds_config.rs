use crate::config::LogLevel;

  
pub fn inject_config_if_allowed(enable_shared_memory: bool, shared_memory_log: LogLevel) {
    let config_string = format!(
        r###"<?xml version="1.0" encoding="UTF-8" ?>
    <CycloneDDS xmlns="https://cdds.io/config"
                xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
                xsi:schemaLocation="https://cdds.io/config https://raw.githubusercontent.com/eclipse-cyclonedds/cyclonedds/iceoryx/etc/cyclonedds.xsd">
        <Domain id="any">
            <SharedMemory>
                <Enable>{}</Enable>
                <LogLevel>{}</LogLevel>
            </SharedMemory>
            <!--
            <Tracing>
                <Category>trace</Category>
                <OutputFile>cdds.log.${{CYCLONEDDS_PID}}</OutputFile>
            </Tracing>
            -->
        </Domain>
    </CycloneDDS>
    "###,
        if enable_shared_memory {
            "true"
        } else {
            "false"
        },
        shared_memory_log.as_str()
    ); 

    // Inject variable only if it was not previously set
    if let Err(_) = std::env::var("CYCLONEDDS_URI") {
        std::env::set_var("CYCLONEDDS_URI", config_string);
    }
}
