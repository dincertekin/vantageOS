//! Built-in shell commands. To add one: write a `cmd_*` fn, add an arm to
//! `dispatch`'s match. No registration, no traits.

use crate::{print, println};

/// Splits `line` into a command name + args, then runs it.
pub fn dispatch(line: &str) {
    let mut words = line.split_whitespace();

    let Some(command) = words.next() else {
        return; // blank line
    };

    match command {
        "help" => cmd_help(),
        "echo" => cmd_echo(words),
        "clear" => cmd_clear(),
        "redraw" => cmd_redraw(),
        _ => println!("vantage: command not found: {command}"),
    }
}

fn cmd_help() {
    println!("available commands:");
    println!("  help          show this message");
    println!("  echo [args]   print the given arguments");
    println!("  clear         clear the terminal screen");
    println!("  redraw        repaint the graphical desktop");
}

/// Re-prints `words` space-separated, like a shell's `echo` builtin.
fn cmd_echo<'a>(words: impl Iterator<Item = &'a str>) {
    let mut first = true;
    for word in words {
        if !first {
            print!(" ");
        }
        print!("{word}");
        first = false;
    }
    println!();
}

fn cmd_clear() {
    // ANSI: clear screen + home cursor. The terminal on the other end of
    // the UART interprets these, not us.
    print!("\x1b[2J\x1b[H");
}

/// Re-paints the desktop -- confirms the framebuffer's still alive.
fn cmd_redraw() {
    if crate::framebuffer::init() {
        crate::gui::draw_desktop();
        println!("desktop redrawn");
    } else {
        println!("vantage: no framebuffer available");
    }
}
