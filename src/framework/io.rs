use std::fmt::Arguments;
use std::io::{self, BufRead, Empty, Sink, Stdin, Stdout, Write};

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
            },
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
    fn write_fmt(&mut self, fmt: Arguments<'_>) -> io::Result<()>;
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