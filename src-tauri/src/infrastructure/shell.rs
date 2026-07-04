use std::env;

use tokio::process::Command;

pub fn command_for_shell(cmd: &str) -> Command {
    #[cfg(windows)]
    {
        let mut command = Command::new("cmd");
        command.arg("/C").arg(cmd);
        command
    }

    #[cfg(not(windows))]
    {
        let mut command = Command::new("sh");
        command.arg("-c").arg(cmd);

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt as _;
            command.as_std_mut().process_group(0);
        }

        command
    }
}

pub fn user_shell_program() -> String {
    #[cfg(windows)]
    {
        env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
    }

    #[cfg(not(windows))]
    {
        env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
    }
}
