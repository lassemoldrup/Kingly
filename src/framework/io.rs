use std::fmt::{Arguments, self};
use std::io::{self, BufRead, Empty, Sink, Stdin, Stdout, Write};

use super::log::LOG;

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


pub struct LoggingOutput<O> {
    output: O,
}

impl<O: Output> LoggingOutput<O> {
    pub fn new(output: O) -> Self {
        Self {
            output,
        }
    }
}

impl<O: Output> Output for LoggingOutput<O> {
    fn write_fmt(&mut self, fmt: fmt::Arguments) -> io::Result<()> {
        let mut line = String::from("Engine: ");
        line.write_fmt(fmt)?;
        LOG.append(&line);

        self.output.write_fmt(fmt)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }
}


pub struct LoggingInput<I> {
    input: I,
}

impl<I: Input> LoggingInput<I> {
    pub fn new(input: I) -> Self {
        Self {
            input,
        }
    }
}

impl<I: Input> Input for LoggingInput<I> {
    fn read_line(&mut self) -> io::Result<String> {
        self.input.read_line()
            .map(|line| {
                LOG.append(&(String::from("GUI: ") + &line));
                line
            })
    }
}