
use std::ffi::c_int;
use libc::{EPOLL_CLOEXEC, epoll_create1, epoll_event, close};
use std::io::Error;
use slotmap:: {SlotMap, DefaultKey};
use std::task::Waker;
use std::fs::File;
use std::os::fd::{ FromRawFd, RawFd };


struct MapNode {
    fd: RawFd,  // Not the owner. Owner is the File object
    waker: Option<Waker>,
}

struct IoDriver { 
    epoll: File,
    map: SlotMap<DefaultKey, MapNode>,
}

impl IoDriver {
    fn new() -> Result<Self, std::io::Error> {
        let map: SlotMap<DefaultKey, MapNode>  = SlotMap::new(); 

        let efd = unsafe { 
            epoll_create1(EPOLL_CLOEXEC) 
        };
        if efd == -1 {
            return Result::Err(Error::last_os_error());
        }

        let epoll = unsafe { 
                File::from_raw_fd(efd)
            };

        return Ok(Self {epoll, map});
    }

    fn event_register(& self, fd: &File, mode: Mode) -> Result<(), std::io::Error> {

        let event: epoll_event;
        let efd = self.epoll.as_raw_fd();
        //Add the event to slotMap and get the key

        let node: MapNode = { 
        efd.insert(
    
        match mode {
            READ: => {
                event.events = EPOLLIN;
            },
            WRITE: => {
                event.events = EPOLLOUT;

            },
            RDWR: => {
                event.events = EPOLLIN|EPOLLOUT;
            },
            _: => {
                return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Invalid mode",
                }
            }
            event.data = key;
        }
    }
