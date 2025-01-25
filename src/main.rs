use anyhow::Result;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

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
    let mut buffer = [0u8; 1];
    let mut output = "bits 16\n".to_owned();

    let print = false;

    loop {
        let num_read = reader.read(&mut buffer)?;
        if num_read == 0 {
            break;
        }

        if print {
            print_buffer(&buffer);
        }

        let opcode_6 = buffer[0] >> 2;
        let opcode_4 = buffer[0] >> 4;
        let output_append = match opcode_4 {
            0b1000 => match opcode_6 {
                // register/memory to/from register
                0b100010 => op_100011(buffer[0], &mut reader, print),
                _ => Ok(String::new()),
            },
            0b1100 => match opcode_6 {
                // immediate to register/memory
                0b110001 => op_110001(buffer[0], &mut reader, print),
                _ => Ok(String::new()),
            },
            0b1011 => op_1011(buffer[0], &mut reader, print),
            _ => Ok(String::new()),
        }?;

        if print {
            println!("add output: {}", output_append);
        }
        output += &output_append;
    }

    Ok(output)
}

fn op_100011(opcode: u8, reader: &mut BufReader<File>, print: bool) -> Result<String> {
    let mut buffer = [0u8; 1];
    reader.read(&mut buffer)?;

    let w = opcode & 0b00000001;
    let direction = (opcode & 0b00000010) >> 1;
    let mode = (buffer[0] & 0b11000000) >> 6;
    let reg = (buffer[0] & 0b00111000) >> 3;
    let rm = buffer[0] & 0b00000111;

    if print {
        println!(
            "opcode: {:06b}. direction {:01b}, wide: {:01b}",
            opcode, direction, w
        );
    }
    if print {
        println!("mod: {:02b}. reg {:03b}, rm: {:03b}", mode, reg, rm);
    }

    let register = get_register(reg, w);
    match mode {
        // memory mode no displacement unless rm=110 (direct address)
        0b00 => {
            let effective_address = effective_address(mode, rm);
            if rm == 0b110 {
                let displacement = get_next_u16(reader)?;
                if direction == 1 {
                    Ok(format!(
                        "\nmov {}, [{} + {}]",
                        register, effective_address, displacement
                    ))
                } else {
                    Ok(format!(
                        "\nmov [{} + {}], {}",
                        effective_address, displacement, register
                    ))
                }
            } else {
                if direction == 1 {
                    Ok(format!("\nmov {}, [{}]", register, effective_address))
                } else {
                    Ok(format!("\nmov [{}], {}", effective_address, register))
                }
            }
        }
        // memory mode 8-bit displacement
        0b01 => {
            let effective_address = effective_address(mode, rm);
            let displacement = get_next_u8(reader)?;
            if direction == 1 {
                Ok(format!(
                    "\nmov {}, [{} + {}]",
                    register, effective_address, displacement
                ))
            } else {
                Ok(format!(
                    "\nmov [{} + {}], {}",
                    effective_address, displacement, register
                ))
            }
        }
        // memory mode 16-bit displacement
        0b10 => {
            let effective_address = effective_address(mode, rm);
            let displacement = get_next_u16(reader)?;
            if direction == 1 {
                Ok(format!(
                    "\nmov {}, [{} + {}]",
                    register, effective_address, displacement
                ))
            } else {
                Ok(format!(
                    "\nmov [{} + {}], {}",
                    effective_address, displacement, register
                ))
            }
        }
        // register mode, no displacement
        0b11 => {
            let register_a = get_register(rm, w);
            let register_b = get_register(reg, w);
            Ok(format!("\nmov {}, {}", register_a, register_b))
        }
        _ => unreachable!(),
    }
}

fn op_110001(opcode: u8, reader: &mut BufReader<File>, print: bool) -> Result<String> {
    let mut buffer = [0u8; 1];
    reader.read(&mut buffer)?;

    let w = opcode & 0b00000001;
    let mode = (buffer[0] & 0b11000000) >> 6;
    let rm = buffer[0] & 0b00000111;
    let effective_address = effective_address(mode, rm);

    match mode {
        // memory mode no displacement unless rm=110 (direct address)
        0b00 => {
            if rm == 0b110 {
                let displacement = get_next_u16(reader)?;
                let immediate_value = match w {
                    0 => get_next_u8(reader)? as u16,
                    1 => get_next_u16(reader)?,
                    _ => unreachable!(),
                };
                Ok(format!(
                    "\nmov [{} + {}], {}",
                    effective_address, displacement, immediate_value
                ))
            } else {
                let immediate_value = match w {
                    0 => get_next_u8(reader)? as u16,
                    1 => get_next_u16(reader)?,
                    _ => unreachable!(),
                };
                Ok(format!(
                    "\nmov [{}], {}",
                    effective_address, immediate_value
                ))
            }
        }
        // memory mode 8-bit displacement
        0b01 => {
            let displacement = get_next_u8(reader)?;
            let immediate_value = match w {
                0 => get_next_u8(reader)? as u16,
                1 => get_next_u16(reader)?,
                _ => unreachable!(),
            };
            Ok(format!(
                "\nmov [{} + {}], {}",
                effective_address, displacement, immediate_value
            ))
        }
        // memory mode 16-bit displacement
        0b10 => {
            let displacement = get_next_u16(reader)?;
            let immediate_value = match w {
                0 => get_next_u8(reader)? as u16,
                1 => get_next_u16(reader)?,
                _ => unreachable!(),
            };
            Ok(format!(
                "\nmov [{} + {}], {}",
                effective_address, displacement, immediate_value
            ))
        }
        // register mode, no displacement
        0b11 => {
            let register = get_register(rm, w);
            let immediate_value = match w {
                0 => get_next_u8(reader)? as u16,
                1 => get_next_u16(reader)?,
                _ => unreachable!(),
            };
            Ok(format!("\nmov {}, {}", register, immediate_value))
        }
        _ => unreachable!(),
    }
}

fn op_1011(opcode: u8, reader: &mut BufReader<File>, print: bool) -> Result<String> {
    let w = (opcode & 0b00001000) >> 3;
    let reg = opcode & 0b00000111;

    let register = get_register(reg, w);
    let immediate_value = match w {
        0 => get_next_u8(reader)? as u16,
        1 => get_next_u16(reader)?,
        _ => unreachable!(),
    };
    Ok(format!("\nmov {}, {}", register, immediate_value))
}

fn get_next_u8(reader: &mut BufReader<File>) -> Result<u8> {
    let mut buffer = [0u8; 1];
    reader.read(&mut buffer)?;
    Ok(u8::from_le_bytes(buffer))
}

fn get_next_u16(reader: &mut BufReader<File>) -> Result<u16> {
    let mut buffer = [0u8; 2];
    reader.read(&mut buffer)?;
    Ok(u16::from_le_bytes(buffer))
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

fn effective_address(mode: u8, rm: u8) -> &'static str {
    match (mode << 3) | rm {
        0b00000 => "bx + si",
        0b00001 => "bx + di",
        0b00010 => "bp + si",
        0b00011 => "bp + di",
        0b00100 => "si",
        0b00101 => "di",
        0b00110 => unreachable!(),
        0b00111 => "bx",

        0b01000 => "bx + si",
        0b01001 => "bx + di",
        0b01010 => "bp + si",
        0b01011 => "bp + di",
        0b01100 => "si",
        0b01101 => "di",
        0b01110 => "bp",
        0b01111 => "bx",

        0b10000 => "bx + si",
        0b10001 => "bx + di",
        0b10010 => "bp + si",
        0b10011 => "bp + di",
        0b10100 => "si",
        0b10101 => "di",
        0b10110 => "bp",
        0b10111 => "bx",

        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_listing_37() {
        let result = process_file("data/listing_0037_single_register_mov").unwrap();
        assert_eq!(result, "bits 16\n\nmov cx, bx".to_owned());
    }

    #[test]
    fn test_listing_38() {
        let result = process_file("data/listing_0038_many_register_mov").unwrap();
        assert_eq!(result,
            "bits 16\n\nmov cx, bx\nmov ch, ah\nmov dx, bx\nmov si, bx\nmov bx, di\nmov al, cl\nmov ch, ch\nmov bx, ax\nmov bx, si\nmov sp, di\nmov bp, ax".to_owned());
    }

    #[test]
    fn test_listing_39() {
        let result = process_file("data/listing_0039_more_movs").unwrap();
        assert_eq!(result,
        "bits 16\n\nmov si, bx\nmov dh, al\nmov cl, 12\nmov ch, 244\nmov cx, 12\nmov cx, 65524\nmov dx, 3948\nmov dx, 61588\nmov al, [bx + si]\nmov bx, [bp + di]\nmov dx, [bp + 0]\nmov ah, [bx + si + 4]\nmov al, [bx + si + 4999]\nmov [bx + di], cx\nmov [bp + si], cl\nmov [bp + 0], ch")
    }
}
