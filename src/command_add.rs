use crate::config_file::{load_mut_config_db, save_config_db, JuliaupConfigChannel};
use crate::global_paths::GlobalPaths;
#[cfg(not(windows))]
use crate::operations::create_symlink;
use crate::operations::{install_version, update_version_db, install_nightly};
use crate::versions_file::load_versions_db;
use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;

pub fn run_command_add(channel: &str, paths: &GlobalPaths) -> Result<()> {
    if channel == "nightly" {
        return add_nightly(paths);
    }

    update_version_db(paths).with_context(|| "Failed to update versions db.")?;
    let version_db =
        load_versions_db(paths).with_context(|| "`add` command failed to load versions db.")?;

    let required_version = &version_db
        .available_channels
        .get(channel)
        .ok_or_else(|| {
            anyhow!(
                "'{}' is not a valid Julia version or channel name.",
                &channel
            )
        })?
        .version;

    let mut config_file = load_mut_config_db(paths)
        .with_context(|| "`add` command failed to load configuration data.")?;

    if config_file.data.installed_channels.contains_key(channel) {
        bail!("'{}' is already installed.", &channel);
    }

    install_version(required_version, &mut config_file.data, &version_db, paths)?;

    config_file.data.installed_channels.insert(
        channel.to_string(),
        JuliaupConfigChannel::SystemChannel {
            version: required_version.clone(),
        },
    );

    if config_file.data.default.is_none() {
        config_file.data.default = Some(channel.to_string());
    }

    #[cfg(not(windows))]
    let create_symlinks = config_file.data.settings.create_channel_symlinks;

    save_config_db(&mut config_file).with_context(|| {
        format!(
            "Failed to save configuration file from `add` command after '{}' was installed.",
            channel
        )
    })?;

    #[cfg(not(windows))]
    if create_symlinks {
        create_symlink(
            &JuliaupConfigChannel::SystemChannel {
                version: required_version.clone(),
            },
            &format!("julia-{}", channel),
            paths,
        )?;
    }

    Ok(())
}

fn add_nightly(paths: &GlobalPaths) -> Result<()> {
    let mut config_db = load_mut_config_db(paths)
        .with_context(|| "`add` command failed to load configuration data.")?;

    install_nightly(&mut config_db.data, paths).with_context(|| {
        format!("Failed to install 'nightly' while updating channel 'nightly'.")
    })?;

    let config_channel = JuliaupConfigChannel::NightlyChannel { last_update: Utc::now() };
    config_db.data.installed_channels.insert(
        "nightly".to_string(),
        config_channel.clone(),
    );
    log::debug!("config_db: {:#?}", config_db);

    save_config_db(&mut config_db).with_context(|| {
        format!(
            "Failed to save configuration file from `add` command after 'nightly' was installed.",
        )
    })?;

    #[cfg(not(windows))]
    if config_db.data.settings.create_channel_symlinks {
        create_symlink(
            &config_channel,
            &config_db.data.settings.nightly_name,
            paths,
        )?;
    }
    Ok(())
}