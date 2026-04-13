use anyhow::{Result, anyhow};

use crate::config::{AudioDeviceConfig, SpiceConfig};

pub(crate) fn validate_audio_devices(
    audio_devices: &[AudioDeviceConfig],
    spice: Option<&SpiceConfig>,
) -> Result<()> {
    validate_spice_audio_requirement(audio_devices, spice)?;

    let mut has_controller = false;
    for audio_device in audio_devices {
        match audio_device.r#type.as_str() {
            "ich9-intel-hda" => {
                validate_audio_controller(audio_device)?;
                has_controller = true;
            }
            "hda-micro" | "hda-duplex" => validate_audio_codec(audio_device)?,
            other => {
                return Err(anyhow!(
                    "Unsupported audio device type: {}. Supported: [\"ich9-intel-hda\", \"hda-micro\", \"hda-duplex\"]",
                    other
                ));
            }
        }
    }

    if !audio_devices.is_empty() && !has_controller {
        return Err(anyhow!(
            "Audio device configuration requires an ich9-intel-hda controller"
        ));
    }

    Ok(())
}

fn validate_spice_audio_requirement(
    audio_devices: &[AudioDeviceConfig],
    spice: Option<&SpiceConfig>,
) -> Result<()> {
    if audio_devices.is_empty() {
        if let Some(spice) = spice
            && spice.enabled
            && spice.audio
        {
            return Err(anyhow!(
                "SPICE audio requires at least one configured audio device"
            ));
        }
        return Ok(());
    }

    match spice {
        Some(spice) if spice.enabled && spice.audio => Ok(()),
        Some(_) => Err(anyhow!(
            "Audio devices currently require spice.enabled=true and spice.audio=true"
        )),
        None => Err(anyhow!(
            "Audio devices currently require a SPICE configuration with audio enabled"
        )),
    }
}

fn validate_audio_controller(audio_device: &AudioDeviceConfig) -> Result<()> {
    if audio_device.id.trim().is_empty() {
        return Err(anyhow!("Audio controller ID cannot be empty"));
    }

    if audio_device.cad.is_some() {
        return Err(anyhow!(
            "Audio controller '{}' cannot define cad",
            audio_device.id
        ));
    }

    if audio_device.audiodev.is_some() {
        return Err(anyhow!(
            "Audio controller '{}' cannot define audiodev",
            audio_device.id
        ));
    }

    Ok(())
}

fn validate_audio_codec(audio_device: &AudioDeviceConfig) -> Result<()> {
    if audio_device.id.trim().is_empty() {
        return Err(anyhow!("Audio codec ID cannot be empty"));
    }

    if audio_device.bus.as_deref().unwrap_or("").trim().is_empty() {
        return Err(anyhow!(
            "Audio codec '{}' requires a bus assignment",
            audio_device.id
        ));
    }

    if audio_device
        .audiodev
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        return Err(anyhow!(
            "Audio codec '{}' requires an audiodev backend ID",
            audio_device.id
        ));
    }

    if audio_device.cad.is_none() {
        return Err(anyhow!(
            "Audio codec '{}' requires a cad value",
            audio_device.id
        ));
    }

    Ok(())
}
