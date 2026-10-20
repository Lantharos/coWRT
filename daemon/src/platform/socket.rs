use std::io;

pub fn option<T>(fd: i32, level: i32, name: i32, value: &T) -> io::Result<()> {
	let result = unsafe { libc::setsockopt(fd, level, name, (value as *const T).cast(), size_of::<T>() as libc::socklen_t) };
	if result < 0 {
		return Err(io::Error::last_os_error());
	}
	Ok(())
}
