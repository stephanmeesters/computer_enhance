use anyhow::Result;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::env;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        panic!("give binary path");
    }
    let result = process_file(&args[1])?;
    println!("{}", result);

    Ok(())
}

fn process_file(filepath: &str) -> Result<String> {
    let f = File::open(filepath)?;
    let mut reader = BufReader::new(f);

    let mut buffer = [0u8; 2];
    let mut output = String::new();
    output += "bits 16\n";

    loop {
        let num_read = reader.read(&mut buffer)?;
        if num_read == 0 {
            break;
        }
        // print_buffer(&buffer);

        let opcode = buffer[0] >> 2;
        let direction = (buffer[0] & 0b00000010) >> 1;
        let w = buffer[0] & 0b00000001;
        // println!("opcode: {:06b}. direction {:01b}, word: {:01b}", opcode, direction, w);

        let modd = (buffer[1] & 0b11000000) >> 6;
        let reg = (buffer[1] & 0b00111000) >> 3;
        let rm = buffer[1] & 0b00000111;
        // println!("mod: {:02b}. reg {:03b}, rm: {:03b}", modd, reg, rm);

        if opcode == 0b100010 {
            let register_a = get_register(rm, w);
            let register_b = get_register(reg, w);
            let out = format!("\nmov {}, {}", register_a, register_b);
            output += &out;
        }
    }

    Ok(output)
}

fn print_buffer(buffer: &[u8]) {
    for u in buffer.iter() {
        print!("{:08b},", u);
    }
    println!();
}

fn get_register(rm: u8, w: u8) -> &'static str {
    match (rm << 1) | w {
        0b0000 => "al",
        0b0001 => "ax",
        0b0010 => "cl",
        0b0011 => "cx",
        0b0100 => "dl",
        0b0101 => "dx",
        0b0110 => "bl",
        0b0111 => "bx",
        0b1000 => "ah",
        0b1001 => "sp",
        0b1010 => "ch",
        0b1011 => "bp",
        0b1100 => "dh",
        0b1101 => "si",
        0b1110 => "bh",
        0b1111 => "di",
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_register() {
        let result = process_file("data/listing_0037_single_register_mov").unwrap();
        assert_eq!(result,
            "bits 16\n\nmov cx, bx".to_owned());
    }

    #[test]
    fn test_many_register() {
        let result = process_file("data/listing_0038_many_register_mov").unwrap();
        assert_eq!(result,
            "bits 16\n\nmov cx, bx\nmov ch, ah\nmov dx, bx\nmov si, bx\nmov bx, di\nmov al, cl\nmov ch, ch\nmov bx, ax\nmov bx, si\nmov sp, di\nmov bp, ax".to_owned());
    }
}
