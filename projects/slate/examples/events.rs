#![feature(allocator_api)]

extern crate alloc;

use core::fmt::Display;

use derive_more::Display;

use slate::event::EventStack;
use slate::draw::Bump;

#[derive(Debug, Display)]
#[display("{:}({:})", self.as_role, self.user_id)]
pub struct UserAuthSuccess<R: Display> {
    pub user_id: u32,
    pub as_role: R,
}

#[derive(Debug, Display)]
pub struct Admin;

#[derive(Debug, Display)]
pub struct User;

#[derive(Debug, Display)]
pub struct Guest;

#[derive(Debug, Display)]
#[display("Downloaded {:} bytes to {:}", self.size_bytes, self.file_path)]
pub struct FileDownloadSuccess<P: Display> {
    size_bytes: u64,
    file_path: P,
}

fn main() {
    println!("Setting up event system for a single scope");
    
    let arena = Bump::new();
    
    let mut event_stack = EventStack::new_in(&arena);
    
    event_stack.push(|event: &UserAuthSuccess<Admin>| {
        println!(" --> {0}", event);
    });
    
    event_stack.push(|event: &UserAuthSuccess<User>| {
        println!(" --> {0}", event);
    });
    
    event_stack.push(|event: &UserAuthSuccess<Guest>| {
        println!(" --> {0}: Audit", event);
    });
    
    event_stack.push(|event: &FileDownloadSuccess<&str>| {
        println!(" --> {0}", event);
    });
    
    //--
    println!("Dispatching UserLoggedIn event");
    
    event_stack.exec(&UserAuthSuccess {
        user_id: 1031,
        as_role: Admin,
    });
    
    event_stack.exec(&UserAuthSuccess {
        user_id: 4693,
        as_role: User,
    });
    
    event_stack.exec(&UserAuthSuccess {
        user_id: 3102,
        as_role: Guest,
    });
    
    //--
    println!("Dispatching FileDownloaded events ..");
    
    event_stack.exec(&FileDownloadSuccess {
        file_path: "/data/report.pdf",
        size_bytes: 4096,
    });
    
    event_stack.exec(&FileDownloadSuccess {
        file_path: "/data/f2-asdfsdf.pdf",
        size_bytes: 391,
    });
    
    event_stack.exec(&FileDownloadSuccess {
        file_path: "/data/final_final_report_v3.3.1.pdf",
        size_bytes: 824040,
    });

    println!("End of scope");
}
