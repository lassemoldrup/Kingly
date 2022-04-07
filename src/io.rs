use std::fmt::Arguments;
use std::io::{self, BufRead, Empty, Sink, Stdin, Stdout, Write};

use log::{debug, Level, RecordBuilder};

pub trait Input {
    fn read_line(&mut self) -> io::Result<String>;
}

impl Input for Stdin {
    fn read_line(&mut self) -> io::Result<String> {
        let mut buf = String::new();
        Stdin::read_line(self, &mut buf)?;
        Ok(buf)
    }
}

impl Input for String {
    fn read_line(&mut self) -> io::Result<String> {
        match self.split_once('\n') {
            None => {
                let mut res = String::new();
                std::mem::swap(&mut res, self);
                res.push('\n');
                Ok(res)
            }
            Some((line, rest)) => {
                let mut res = line.to_string();
                res.push('\n');
                *self = rest.to_string();
                Ok(res)
            }
        }
    }
}

impl Input for Empty {
    fn read_line(&mut self) -> io::Result<String> {
        let mut buf = String::new();
        BufRead::read_line(self, &mut buf)?;
        Ok(buf)
    }
}

pub trait Output {
    fn write_fmt(&mut self, fmt: Arguments) -> io::Result<()>;
    fn flush(&mut self) -> io::Result<()>;
}

impl Output for Stdout {
    fn write_fmt(&mut self, fmt: Arguments<'_>) -> io::Result<()> {
        Write::write_fmt(self, fmt)
    }

    fn flush(&mut self) -> io::Result<()> {
        Write::flush(self)
    }
}

impl Output for String {
    fn write_fmt(&mut self, fmt: Arguments<'_>) -> io::Result<()> {
        self.push_str(&std::fmt::format(fmt));
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Output for Sink {
    fn write_fmt(&mut self, fmt: Arguments<'_>) -> io::Result<()> {
        Write::write_fmt(self, fmt)
    }

    fn flush(&mut self) -> io::Result<()> {
        Write::flush(self)
    }
}

impl<O: Output> Output for &mut O {
    fn write_fmt(&mut self, fmt: Arguments<'_>) -> io::Result<()> {
        (*self).write_fmt(fmt)
    }

    fn flush(&mut self) -> io::Result<()> {
        (*self).flush()
    }
}

pub struct Logging<T>(pub T);

impl<T: Input> Input for Logging<T> {
    fn read_line(&mut self) -> io::Result<String> {
        self.0.read_line().map(|s| {
            debug!("{}", s);
            s
        })
    }
}

impl<T: Output> Output for Logging<T> {
    fn write_fmt(&mut self, fmt: Arguments) -> io::Result<()> {
        let log_record = RecordBuilder::new().level(Level::Debug).args(fmt).build();
        log::logger().log(&log_record);
        self.0.write_fmt(fmt)
    }

    fn flush(&mut self) -> io::Result<()> {
        log::logger().flush();
        self.0.flush()
    }
}
