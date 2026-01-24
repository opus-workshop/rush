use crate::executor::{ExecutionResult, Output};
use crate::runtime::Runtime;
use crate::jobs::{Job, JobStatus};
use anyhow::{anyhow, Result};

/// List all background jobs
pub fn builtin_jobs(args: &[String], runtime: &mut Runtime) -> Result<ExecutionResult> {
    let mut output = String::new();

    // Update job statuses before listing
    runtime.job_manager().update_all_jobs();

    // Parse options
    let mut show_pids = false;
    let mut show_running = false;
    let mut show_stopped = false;

    for arg in args {
        match arg.as_str() {
            "-l" => show_pids = true,
            "-r" => show_running = true,
            "-s" => show_stopped = true,
            _ => {
                return Err(anyhow!("jobs: invalid option: {}", arg));
            }
        }
    }

    // Get all jobs
    let jobs = runtime.job_manager().list_jobs();

    if jobs.is_empty() {
        return Ok(ExecutionResult::success(String::new()));
    }

    for job in jobs {
        // Filter by status if requested
        if show_running && job.status != JobStatus::Running {
            continue;
        }
        if show_stopped && job.status != JobStatus::Stopped {
            continue;
        }

        // Format output
        if show_pids {
            output.push_str(&format!(
                "[{}] {} {}\t{}\n",
                job.id,
                job.status.as_str(),
                job.pid,
                job.command
            ));
        } else {
            output.push_str(&format!(
                "[{}] {}\t{}\n",
                job.id,
                job.status.as_str(),
                job.command
            ));
        }
    }

    Ok(ExecutionResult::success(output))
}

/// Bring a job to foreground
pub fn builtin_fg(args: &[String], runtime: &mut Runtime) -> Result<ExecutionResult> {
    use nix::sys::wait::{waitpid, WaitPidFlag};
    use nix::unistd::Pid;

    // Update job statuses
    runtime.job_manager().update_all_jobs();

    // Determine which job to foreground
    let job = if args.is_empty() {
        // Default to current job (most recent)
        runtime.job_manager().get_current_job()
            .ok_or_else(|| anyhow!("fg: no current job"))?
    } else {
        let job_spec = &args[0];
        parse_job_spec(job_spec, runtime)?
    };

    let job_id = job.id;
    let pid = job.pid;

    // If the job is stopped, continue it
    if job.status == JobStatus::Stopped {
        runtime.job_manager().continue_job(job_id)
            .map_err(|e: String| anyhow!(e))?;
    }

    // Remove from job list
    runtime.job_manager().remove_job(job_id);

    // Wait for the process to complete in foreground
    let pid = Pid::from_raw(pid as i32);

    // Print the command being foregrounded
    eprintln!("{}", job.command);

    match waitpid(pid, Some(WaitPidFlag::WUNTRACED)) {
        Ok(status) => {
            use nix::sys::wait::WaitStatus;
            let exit_code = match status {
                WaitStatus::Exited(_, code) => code,
                WaitStatus::Signaled(_, _, _) => 128 + 15, // SIGTERM
                WaitStatus::Stopped(_, _) => {
                    // Job was stopped again, add it back
                    runtime.job_manager().add_job(pid.as_raw() as u32, job.command.clone());
                    1
                }
                _ => 0,
            };
            Ok(ExecutionResult {
                output: Output::Text(String::new()),
                stderr: String::new(),
                exit_code,
                error: None,
            })
        }
        Err(e) => Err(anyhow!("fg: failed to wait for job: {}", e)),
    }
}

/// Continue a stopped job in background
pub fn builtin_bg(args: &[String], runtime: &mut Runtime) -> Result<ExecutionResult> {
    // Update job statuses
    runtime.job_manager().update_all_jobs();

    // Determine which job to background
    let job = if args.is_empty() {
        // Default to current job (most recent)
        runtime.job_manager().get_current_job()
            .ok_or_else(|| anyhow!("bg: no current job"))?
    } else {
        let job_spec = &args[0];
        parse_job_spec(job_spec, runtime)?
    };

    // Only continue if stopped
    if job.status != JobStatus::Stopped {
        return Err(anyhow!("bg: job {} is not stopped", job.id));
    }

    // Continue the job
    runtime.job_manager().continue_job(job.id)
        .map_err(|e: String| anyhow!(e))?;

    Ok(ExecutionResult::success(format!("[{}] {}\n", job.id, job.command)))
}

/// Parse job specification (e.g., %1, %%, %+, %-, %sleep)
fn parse_job_spec(spec: &str, runtime: &Runtime) -> Result<Job> {
    if !spec.starts_with('%') {
        // Try parsing as job ID
        if let Ok(job_id) = spec.parse::<usize>() {
            return runtime.job_manager().get_job(job_id)
                .ok_or_else(|| anyhow!("No such job: {}", job_id));
        }
        return Err(anyhow!("Invalid job specification: {}", spec));
    }

    let spec = &spec[1..]; // Remove %

    match spec {
        "" | "%" | "+" => {
            // Current job
            runtime.job_manager().get_current_job()
                .ok_or_else(|| anyhow!("No current job"))
        }
        "-" => {
            // Previous job
            runtime.job_manager().get_previous_job()
                .ok_or_else(|| anyhow!("No previous job"))
        }
        _ => {
            // Try parsing as job ID
            if let Ok(job_id) = spec.parse::<usize>() {
                runtime.job_manager().get_job(job_id)
                    .ok_or_else(|| anyhow!("No such job: {}", job_id))
            } else {
                // Try matching by command prefix
                let jobs = runtime.job_manager().list_jobs();
                let matching: Vec<_> = jobs.into_iter()
                    .filter(|j| j.command.starts_with(spec))
                    .collect();

                match matching.len() {
                    0 => Err(anyhow!("No such job: %{}", spec)),
                    1 => Ok(matching[0].clone()),
                    _ => Err(anyhow!("Ambiguous job specification: %{}", spec)),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jobs_empty() {
        let mut runtime = Runtime::new();
        let result = builtin_jobs(&[], &mut runtime).unwrap();
        assert_eq!(result.stdout(), "");
    }

    #[test]
    fn test_jobs_with_job() {
        let mut runtime = Runtime::new();
        // Add a fake job
        runtime.job_manager().add_job(12345, "sleep 100".to_string());

        let result = builtin_jobs(&[], &mut runtime).unwrap();
        assert!(result.stdout().contains("[1]"));
        assert!(result.stdout().contains("sleep 100"));
    }

    #[test]
    fn test_jobs_with_pid_flag() {
        let mut runtime = Runtime::new();
        runtime.job_manager().add_job(12345, "sleep 100".to_string());

        let result = builtin_jobs(&["-l".to_string()], &mut runtime).unwrap();
        assert!(result.stdout().contains("12345"));
    }
}
