use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use nix::sys::signal::{kill, Signal};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Running,
    Stopped,
    Done,
    Terminated,
}

impl JobStatus {
    pub fn as_str(&self) -> &str {
        match self {
            JobStatus::Running => "Running",
            JobStatus::Stopped => "Stopped",
            JobStatus::Done => "Done",
            JobStatus::Terminated => "Terminated",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Job {
    pub id: usize,
    pub pid: u32,
    pub pgid: u32, // Process group ID
    pub command: String,
    pub status: JobStatus,
}

impl Job {
    pub fn new(id: usize, pid: u32, command: String) -> Self {
        Self {
            id,
            pid,
            pgid: pid, // Initially, pgid == pid (process becomes group leader)
            command,
            status: JobStatus::Running,
        }
    }

    /// Update job status by checking the process
    pub fn update_status(&mut self) -> Result<(), String> {
        let pid = Pid::from_raw(self.pid as i32);

        match waitpid(pid, Some(WaitPidFlag::WNOHANG | WaitPidFlag::WUNTRACED)) {
            Ok(WaitStatus::Exited(_, _)) => {
                self.status = JobStatus::Done;
            }
            Ok(WaitStatus::Signaled(_, _, _)) => {
                self.status = JobStatus::Terminated;
            }
            Ok(WaitStatus::Stopped(_, _)) => {
                self.status = JobStatus::Stopped;
            }
            Ok(WaitStatus::StillAlive) => {
                // Process is still running
                if self.status == JobStatus::Stopped {
                    // Keep stopped status
                } else {
                    self.status = JobStatus::Running;
                }
            }
            Ok(_) => {
                // Other statuses, keep current
            }
            Err(_) => {
                // Process doesn't exist or error checking
                self.status = JobStatus::Done;
            }
        }

        Ok(())
    }

    /// Send SIGCONT to continue a stopped job
    pub fn continue_job(&mut self) -> Result<(), String> {
        let pid = Pid::from_raw(self.pid as i32);
        kill(pid, Signal::SIGCONT)
            .map_err(|e| format!("Failed to continue job {}: {}", self.id, e))?;
        self.status = JobStatus::Running;
        Ok(())
    }

    /// Send SIGTERM to terminate the job
    pub fn terminate(&mut self) -> Result<(), String> {
        let pid = Pid::from_raw(self.pid as i32);
        kill(pid, Signal::SIGTERM)
            .map_err(|e| format!("Failed to terminate job {}: {}", self.id, e))?;
        self.status = JobStatus::Terminated;
        Ok(())
    }
}

/// Job manager for tracking background jobs
#[derive(Clone)]
pub struct JobManager {
    jobs: Arc<Mutex<HashMap<usize, Job>>>,
    next_job_id: Arc<Mutex<usize>>,
}

impl JobManager {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(HashMap::new())),
            next_job_id: Arc::new(Mutex::new(1)),
        }
    }

    /// Add a new background job
    pub fn add_job(&self, pid: u32, command: String) -> usize {
        let mut next_id = self.next_job_id.lock().unwrap();
        let job_id = *next_id;
        *next_id += 1;
        drop(next_id);

        let job = Job::new(job_id, pid, command);
        let mut jobs = self.jobs.lock().unwrap();
        jobs.insert(job_id, job);

        job_id
    }

    /// Get a job by ID
    pub fn get_job(&self, job_id: usize) -> Option<Job> {
        let jobs = self.jobs.lock().unwrap();
        jobs.get(&job_id).map(|j| Job {
            id: j.id,
            pid: j.pid,
            pgid: j.pgid,
            command: j.command.clone(),
            status: j.status,
        })
    }

    /// Get a job by PID
    pub fn get_job_by_pid(&self, pid: u32) -> Option<Job> {
        let jobs = self.jobs.lock().unwrap();
        jobs.values()
            .find(|j| j.pid == pid)
            .map(|j| Job {
                id: j.id,
                pid: j.pid,
                pgid: j.pgid,
                command: j.command.clone(),
                status: j.status,
            })
    }

    /// List all jobs
    pub fn list_jobs(&self) -> Vec<Job> {
        let jobs = self.jobs.lock().unwrap();
        jobs.values()
            .map(|j| Job {
                id: j.id,
                pid: j.pid,
                pgid: j.pgid,
                command: j.command.clone(),
                status: j.status,
            })
            .collect()
    }

    /// Update all job statuses
    pub fn update_all_jobs(&self) {
        let mut jobs = self.jobs.lock().unwrap();
        for job in jobs.values_mut() {
            let _ = job.update_status();
        }
    }

    /// Remove completed/terminated jobs
    pub fn cleanup_jobs(&self) {
        let mut jobs = self.jobs.lock().unwrap();
        jobs.retain(|_, job| {
            job.status != JobStatus::Done && job.status != JobStatus::Terminated
        });
    }

    /// Bring a job to foreground (remove from job list)
    pub fn remove_job(&self, job_id: usize) -> Option<Job> {
        let mut jobs = self.jobs.lock().unwrap();
        jobs.remove(&job_id)
    }

    /// Continue a stopped job in background
    pub fn continue_job(&self, job_id: usize) -> Result<(), String> {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(job) = jobs.get_mut(&job_id) {
            job.continue_job()
        } else {
            Err(format!("Job {} not found", job_id))
        }
    }

    /// Terminate a job
    pub fn terminate_job(&self, job_id: usize) -> Result<(), String> {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(job) = jobs.get_mut(&job_id) {
            job.terminate()
        } else {
            Err(format!("Job {} not found", job_id))
        }
    }

    /// Get the most recent job (for %+ or %% syntax)
    pub fn get_current_job(&self) -> Option<Job> {
        let jobs = self.jobs.lock().unwrap();
        jobs.values()
            .max_by_key(|j| j.id)
            .map(|j| Job {
                id: j.id,
                pid: j.pid,
                pgid: j.pgid,
                command: j.command.clone(),
                status: j.status,
            })
    }

    /// Get the previous job (for %- syntax)
    pub fn get_previous_job(&self) -> Option<Job> {
        let jobs = self.jobs.lock().unwrap();
        let mut job_list: Vec<_> = jobs.values().collect();
        job_list.sort_by_key(|j| j.id);

        if job_list.len() >= 2 {
            let prev = job_list[job_list.len() - 2];
            Some(Job {
                id: prev.id,
                pid: prev.pid,
                pgid: prev.pgid,
                command: prev.command.clone(),
                status: prev.status,
            })
        } else {
            None
        }
    }
}

impl Default for JobManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_manager_add_job() {
        let manager = JobManager::new();
        let job_id = manager.add_job(1234, "sleep 100".to_string());
        assert_eq!(job_id, 1);

        let job = manager.get_job(job_id).unwrap();
        assert_eq!(job.pid, 1234);
        assert_eq!(job.command, "sleep 100");
        assert_eq!(job.status, JobStatus::Running);
    }

    #[test]
    fn test_job_manager_list_jobs() {
        let manager = JobManager::new();
        manager.add_job(1234, "sleep 100".to_string());
        manager.add_job(5678, "cat".to_string());

        let jobs = manager.list_jobs();
        assert_eq!(jobs.len(), 2);
    }

    #[test]
    fn test_job_manager_remove_job() {
        let manager = JobManager::new();
        let job_id = manager.add_job(1234, "sleep 100".to_string());

        let removed = manager.remove_job(job_id);
        assert!(removed.is_some());
        assert!(manager.get_job(job_id).is_none());
    }

    #[test]
    fn test_job_status_as_str() {
        assert_eq!(JobStatus::Running.as_str(), "Running");
        assert_eq!(JobStatus::Stopped.as_str(), "Stopped");
        assert_eq!(JobStatus::Done.as_str(), "Done");
        assert_eq!(JobStatus::Terminated.as_str(), "Terminated");
    }
}
