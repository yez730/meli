use chrono::{DateTime, Local, Duration};

#[derive(Clone,Debug)]
pub struct AxumSessionConfig{
    pub idle_timeout:Duration,
    pub memory_clear_timeout:Duration, 
}

impl Default for AxumSessionConfig{
    fn default() -> Self{
        AxumSessionConfig{
            idle_timeout:Duration::days(300),
            memory_clear_timeout:Duration::minutes(10),
        }
    }
}