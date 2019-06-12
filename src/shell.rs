use std::error::Error;
use std::fmt;
use std::io::Write;
use termcolor::Color::{Green, Red, Yellow};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

pub struct Shell {
    stream: StandardStream,
}

impl Default for Shell {
    fn default() -> Shell {
        Shell {
            stream: StandardStream::stderr(ColorChoice::Always),
        }
    }
}

impl Shell {
    pub fn new() -> Shell {
        Shell::default()
    }

    pub fn warn<T: fmt::Display>(&mut self, message: T) -> Result<(), Box<Error>> {
        self.print(&"warning:", &message, Yellow, false)
    }

    pub fn error<T: fmt::Display>(&mut self, message: T) -> Result<(), Box<Error>> {
        self.print(&"error:", &message, Red, false)
    }

    pub fn status<T, U>(&mut self, status: T, message: U) -> Result<(), Box<Error>>
    where
        T: fmt::Display,
        U: fmt::Display,
    {
        self.print(&status, &message, Green, true)
    }

    fn print(
        &mut self,
        status: &fmt::Display,
        message: &fmt::Display,
        color: Color,
        justifed: bool,
    ) -> Result<(), Box<Error>> {
        self.stream.reset()?;
        self.stream
            .set_color(ColorSpec::new().set_bold(true).set_fg(Some(color)))?;
        if justifed {
            write!(self.stream, "{:>12}", status)?;
        } else {
            write!(self.stream, "{}", status)?;
        }
        self.stream.reset()?;
        writeln!(self.stream, " {}", message)?;

        Ok(())
    }
}
