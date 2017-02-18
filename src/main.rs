#[allow(dead_code)]
extern crate elf;
extern crate unicorn;
extern crate capstone;
extern crate rand;
extern crate getopts;

use getopts::Options;
use std::env;

mod roper;

use rand::{Rng,Generator};

use std::path::PathBuf;
use std::fs::File;
use std::io::prelude::*;
use unicorn::*;
// use std::io;
//use roper::dis::{disas_sec,Inst};
use roper::thumb::*;
use roper::util::*;
use roper::params::*;
use roper::population::*;
use roper::hatchery::{add_hooks,Sec,HatchResult,hatch_chain};
use roper::phylostructs::*;
use roper::evolution::*;
use roper::ontostructs::*;


fn print_usage (program: &str, opts: Options) {
  let brief = format!("Usage: {} [options]", program);
  print!("{}", opts.usage(&brief));
}

fn load_file (path: &str) -> Vec<u8>
{
  let mut f = File::open(path)
                .expect("Failed to open path");
  let mut buf : Vec<u8> = Vec::new();
  f.read_to_end(&mut buf);
  buf
}

fn get_elf_addr_data (path: &str, 
                      secs: &Vec<&str>) 
                      -> Vec<Sec> {
  let path = PathBuf::from(path);
  let file = match elf::File::open_path(&path) {
    Ok(f) => f,
    Err(e) => panic!("Error: {:?}",e),
  };
  let mut sections : Vec<Sec> = Vec::new();
  for sec_name in secs.iter() {
    let sec = file.get_section(sec_name)
                  .expect("Unable to fetch section from elf");
    sections.push(Sec {
      name: sec_name.to_string(),
      addr: sec.shdr.addr,
      data: sec.data.clone(),
      perm: PROT_ALL, // Placeholder. Need to convert from elf
    });
  }
  sections
}

fn get_gba_addr_data (path: &str) -> Vec<(u64, Vec<u8>)> {
  let addr = GBA_CARTRIDGE_ROM_START;
  let data = load_file(path);
  vec![(addr,data)]
}
                    

const GBA_CARTRIDGE_ROM_START : u64 = 0x08000000;

/* Just a debugging stub */
fn main() {
  
  let args: Vec<String> = env::args().collect();
  let program = args[0].clone();

  let mut opts = Options::new();
  opts.optopt("p", "", "set target pattern", "PATTERN");
  opts.optopt("h", "help", "print this help menu", "");
  let matches = match opts.parse(&args[1..]) {
    Ok(m)  => { m },
    Err(f) => { panic!(f.to_string()) },
  };
  if matches.opt_present("h") {
    print_usage(&program, opts);
    return;
  }
  let rpattern_str = match matches.opt_str("p") {
    Some(s) => s,
    None    => "1 _ deba5e12 _ _ 0".to_string(),
  };
  let rpattern : RPattern = RPattern::new(&rpattern_str);

  println!("{:?} => {}", rpattern_str, rpattern);
 
  /**************************************************/
  let sample1 = "tomato-RT-AC3200-ARM-132-AIO-httpd";
  let sample2 = "tomato-RT-N18U-httpd";
  let sample3 = "openssl";
  let sample4 = "ldconfig.real";
  let sample_gba = "megaman_zero_4.gba";
  let sample_root = "/home/oblivia/Projects/roper/data/"
    .to_string();
  let elf_path = sample_root.clone() + sample4;
  let gba_path = sample_root.clone() + sample_gba;
  let elf_addr_data = get_elf_addr_data(&elf_path,
                                        &vec![".text",".rodata"]);
  println!("****************** ELF {} **********************",
           elf_path);
  let text_addr = elf_addr_data[0].addr;
  let text_data = &elf_addr_data[0].data;
  let rodata_addr = elf_addr_data[1].addr;
  let rodata_data = &elf_addr_data[1].data;
  let wordvec_elf = u8s_to_u16s(&text_data, Endian::LITTLE);
  
  let mode = MachineMode::ARM;

  for _ in 0..40 { print!("*"); }
  println!("");

  let elf_clumps = reap_gadgets(text_data,
                                text_addr as u32,
                                mode);

  let mut params : Params = Params::new();
  params.code = text_data.clone();
  params.code_addr = text_addr as u32;
  params.data = vec![rodata_data.clone()];
  params.data_addrs = vec![rodata_addr as u32];
  params.constants  = rpattern.constants();
  params.io_targets = vec![(vec![0; 16], rpattern)];

  let mut rng = rand::thread_rng();
  let mut population = Population::new(&params, &mut rng);

/*  let mut mangler : Mangler = Mangler::new(&params.constants);
  let mut machinery = Machinery { 
                        rng: rand::thread_rng(),
                        uc:  roper::init_engine(&elf_addr_data, mode),
                        mangler: mangler,
                      };
                      */
  let mut machinery = Machinery::new(&elf_path,
                                     mode,
                                     &params.constants);


//  println!("POPULATION SIZE:\n{}", population.size());
/* tournement broken 
  let mut rchain0 = random_chain(&mut elf_clumps,
                                 params.min_start_len,
                                 params.max_start_len,
                                 &mut machinery.mangler,
                                 &mut machinery.rng);
  let mut rchain1 = random_chain(&mut elf_clumps,
                                 params.min_start_len,
                                 params.max_start_len,
                                 &mut machinery.mangler,
                                 &mut machinery.rng); 
  println!("rchain0:\n{}\n", rchain0);
  println!("rchain1:\n{}\n", rchain1);
  println!("about to eval fitness for r0");
  */
  let mut i : usize = 0;
  while population.best_fit() != Some(0) {
    tournement(&mut population, &mut machinery);
    i += 1;
  }
  println!("=> {} GENERATIONS", i);
  println!("=> BEST FIT: {:?}", population.best_fit());
  println!("=> RUNNING BEST:\n");
           
  add_hooks(&mut machinery.uc);
  let mut bclone = population.best.unwrap().clone();
  evaluate_fitness(&mut machinery.uc,
                   &mut bclone,
                   &population.params.io_targets);
  for p in population.params.io_targets.iter() {
    println!("{}", p.1);
  }
  println!("\n{}", &bclone);
  /* 
  let r0 = evaluate_fitness(&mut machinery.uc,
                            &mut rchain0, 
                            &params.io_targets);
  println!("evaluate_fitness for rchain0 -> {:?}", r0);
  println!("rchain0:\n{}\n", rchain0);
  let r1 = evaluate_fitness(&mut machinery.uc, 
                            &mut rchain1, 
                            &params.io_targets);
  println!("evaluate_fitness for rchain1 -> {:?}", r1);
  println!("rchain1:\n{}\n", rchain1);
  println!("*** M A T I N G ***\n");
  let spawn : Vec<Chain> = mate(&vec![&rchain0, &rchain1],
                                &params,
                                &mut machinery.rng,
                                &mut machinery.uc);

  for child in &spawn {
    println!("child:\n{}", child);
  }
  *****/
}
