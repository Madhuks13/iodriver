use libc;
use std::time;
use std::io::{Error, ErrorKind};
use std::fs::File;
use std::os::fd::{FromRawFd, AsRawFd};


static MAX_EVENTS: usize = 64;
//hust for compilation
type EventKey = u64;

//Libc struct to rust
pub struct Event {
    events: u32,
    key: u64,
}

impl Event {
    pub fn is_readable(&self) -> bool {
        self.events & libc::EPOLLIN as u32 != 0 
    }
    pub fn is_writable(&self) -> bool {
        self.events & libc::EPOLLOUT as u32 != 0
    }
    pub fn is_read_closed(&self) -> bool {
        self.events & libc::EPOLLRDHUP as u32 != 0
    }
    pub fn is_error(&self) -> bool {
        self.events & libc::EPOLLERR as u32 != 0
    }
    pub fn is_hangup(&self) -> bool {
        self.events & libc::EPOLLHUP as u32 != 0
    }
    pub fn get_key(&self) -> u64 {
        self.key
    }
}

pub enum Mode {
    Read,
    Write,
    ReadWrite,
}

fn mode_to_events(mode: Mode) -> u32 { 
    let mut events = libc::EPOLLET as u32;
        events |= match mode {
            Mode::Read => libc::EPOLLIN as u32 | libc::EPOLLRDHUP as u32,
            Mode::Write => libc::EPOLLOUT as u32,
            Mode::ReadWrite => libc::EPOLLIN as u32 | libc::EPOLLOUT as u32| libc::EPOLLRDHUP as u32,
        };

    events
}

pub struct IoDriver { 
    epoll: File,
    kernel_events: Vec<libc::epoll_event>, //stores the kernel returned event array
}

impl IoDriver {
    pub fn new() -> Result<Self, Error> {

        let efd = unsafe { 
            libc::epoll_create1(libc::EPOLL_CLOEXEC) 
        };
        if efd == -1 {
            return Err(Error::last_os_error());
        }

        let epoll = unsafe { 
                File::from_raw_fd(efd)
            };


        let kernel_events = vec![libc::epoll_event {events: 0, u64: 0}; MAX_EVENTS];

        Ok(Self {epoll, kernel_events} )
    }

    pub fn io_add<F>(&self, file: &F, mode: Mode, key: EventKey) -> Result<(), Error> 
    where F: AsRawFd,
    {
        let events = mode_to_events(mode);

        let mut event = libc::epoll_event { events, u64: key, };

        let result = unsafe { 
            libc::epoll_ctl(self.epoll.as_raw_fd(), libc::EPOLL_CTL_ADD, file.as_raw_fd(), &mut event)  
        };
        if result == -1 {
            return Err(Error::last_os_error());
        }
            
        Ok(())
    }

    // pre condition - FD already registered. condition should be checked at user level. This avoids system_call
    pub fn io_modify<F>(&self, file: &F, mode: Mode, key: EventKey) -> Result<(), Error>
    where F: AsRawFd,
    {
        let events = mode_to_events(mode);

        let mut event = libc::epoll_event { events, u64: key, };

        let result = unsafe {
            libc::epoll_ctl(self.epoll.as_raw_fd(), libc::EPOLL_CTL_MOD, file.as_raw_fd(), &mut event)
        };
        if result == -1 {
            return Err(Error::last_os_error());
        }

        Ok(())
    }


    // pre condition - FD already registered. condition should be checked at user level. This avoids system_call
    pub fn io_delete<F>(&self, file: &F) -> Result<(), Error>
    where F: AsRawFd,
    {
        let result = unsafe { 
            libc::epoll_ctl(self.epoll.as_raw_fd(), libc::EPOLL_CTL_DEL, file.as_raw_fd(), std::ptr::null_mut::<libc::epoll_event>())
        };
        if result == -1 {
            return Err(Error::last_os_error());
        }

        Ok(())
    }

    pub fn io_wait(&mut self, mut timeout: i32) -> Result<Vec<Event>, Error> {
        loop {
            let start = time::Instant::now();

            let result =  unsafe {
                libc::epoll_wait(self.epoll.as_raw_fd(), self.kernel_events.as_mut_ptr(), MAX_EVENTS as libc::c_int, timeout)
            };

            if result < 0 { 
                //Signal interrupted. call epoll_wait() with remaining timeout
                let error = Error::last_os_error();
                match error.kind() {
                    ErrorKind::Interrupted => {
                        if timeout > 0 { 
                            let elapsed = start.elapsed().as_millis() as i32;
                            timeout = timeout.saturating_sub(elapsed);
                        }
                        continue;                                              
                    }
                    _ => { return Err(error); }
                }                
            }
            else if result == 0 { return Ok(vec![]); }
            else {
                //push the kernel returned evets to self.events
                let events: Vec<Event> = self.kernel_events[..result as usize]
                .iter()
                .map(|e| -> Event {
                    Event {
                        events: e.events,
                        key: e.u64,
                    }
                })
                .collect();
                return Ok(events);
            }   
        }
    }
}
