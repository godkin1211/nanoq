use clap::{Arg, ArgMatches, App};
use std::cmp::Ordering;
use bio::io::fastq;
use std::path::Path;
use std::fs;
use std::io;
use libm::log10;

fn command_line_interface<'a>() -> ArgMatches<'a> {

    // Sets the command line interface of the program.

    App::new("nanoq")
            .version("0.0.1")
            .about("\nMinimal quality control for nanopore reads\n")
            .arg(Arg::from_usage("-f, --fastq=[FILE] 'Input fastq file [required]'"))
            .arg(Arg::from_usage("-o, --output=[FILE] 'Output fastq file [required].'"))
            .arg(Arg::from_usage("-l, --length=[INT] 'Minimum read length [0]'"))
            .arg(Arg::from_usage("-q, --quality=[INT] 'Minimum read quality [0]'"))
            .get_matches()
}

fn main() {

    let cli = command_line_interface();
 
    let reader = match cli.value_of("fastq") {
        Some(filename) => fastq::Reader::from_file(filename).expect("Could not initiate reader from file"),
        None => panic!("Error: input file must be specified with --fastq")
    };

    let path = match cli.value_of("output") {
        Some(filename) => Path::new(filename),
        None => panic!("Error: output file must be specified with --output")
    };
    
    let min_length: u64 = cli.value_of("length").unwrap_or("0").parse().unwrap();
    let min_quality: f64 = cli.value_of("quality").unwrap_or("0").parse().unwrap();

    let file = fs::File::create(path).unwrap();

    let handle = io::BufWriter::new(file);
    let mut writer = fastq::Writer::new(handle);

    let mut basepairs: u64 = 0;
    let mut reads: u64 = 0;
    let mut read_lengths: Vec<u64> = Vec::new();
    let mut read_qualities: Vec<f64> = Vec::new();

    for result in reader.records() {
        
        let record = result.expect("Error: could not parse record");
        
        // Nanopore quality score computation

        let quality_values: Vec<u8> = record.qual().to_vec();
        let mean_error = get_mean_error(&quality_values);
        let mean_quality: f64 = -10f64*log10(mean_error as f64);

        let seq_len = record.seq().len() as u64;

        read_lengths.push(seq_len);
        read_qualities.push(mean_quality);
        
        basepairs += seq_len;
        reads += 1;

        if seq_len >= min_length && mean_quality >= min_quality {
            writer.write_record(&record).expect("Error: could not write record.");
        }           

    }  

    // Summary statistics

    let mean_read_length = get_mean_read_length(&read_lengths);
    let mean_read_quality = get_mean_read_quality(&read_qualities);
    let median_read_length = get_median_read_length(&mut read_lengths);
    let median_read_quality = get_median_read_quality(&mut read_qualities);
    let (min_read_length, max_read_length) = get_read_length_range(&read_lengths);

    eprintln!("{:} {:} {:} {:} {:} {:} {:.2} {:.2}", basepairs, reads, max_read_length, min_read_length, mean_read_length, median_read_length, mean_read_quality, median_read_quality);
    
}

// Helper functions

fn compare_f64(a: &f64, b: &f64) -> Ordering {

    // Will get killed with NAN (R.I.P)
    // but we should also never see NAN

    if a < b {
        return Ordering::Less;
    } else if a > b {
        return Ordering::Greater;
    }
    Ordering::Equal
}

fn get_mean_error(quality_bytes: &Vec<u8>) -> f32 {

    /* Compute the error probability from the quality score of a single base

    Quality encoding: Sanger Phred+33 --> ASCII: 33 - 126 --> Q: 0 - 93

    f32 vs f64 makes a huge difference!

    Computation of the base quality scores is described at:

    https://community.nanoporetech.com/technical_documents/data-analysis/

    https://gigabaseorgigabyte.wordpress.com/2017/06/26/averaging-basecall-quality-scores-the-right-way/

    */
    
    let mut sum: f32 = 0.0;
    for q in quality_bytes.iter(){
        sum += 10f32.powf((q-33u8) as f32 / -10f32)  // Translate to Q-score and error probability
    }
    
    sum / quality_bytes.len() as f32  // Mean error probability

}

// Read length range

fn get_read_length_range(numbers: &Vec<u64>) -> (&u64, &u64) {

    let min_read_length = numbers.iter().min().expect("Could not determine minimum read length");
    let max_read_length = numbers.iter().max().expect("Could not determine maximum read length");
    
    return (min_read_length, max_read_length)

}

// Mean and medians for different numeric types

fn get_median_read_length(numbers: &mut Vec<u64>) -> u64 {
    
    // Compute the median of a vector of unsigned integers

    numbers.sort();

    let mid = numbers.len() / 2;
    if numbers.len() % 2 == 0 {
        get_mean_read_length(&vec![numbers[mid - 1], numbers[mid]]) as u64
    } else {
        numbers[mid]
    }

}

fn get_mean_read_length(numbers: &Vec<u64>) -> u64 {

    // Compute the mean of a vector of unsigned integers

    let sum: u64 = numbers.iter().sum();

    sum as u64 / numbers.len() as u64

}


fn get_median_read_quality(numbers: &mut Vec<f64>) -> f64 {

    // Compute the median of a vector of double-precision floats

    numbers.sort_by(compare_f64);

    let mid = numbers.len() / 2;
    if numbers.len() % 2 == 0 {
        get_mean_read_quality(&vec![numbers[mid - 1], numbers[mid]]) as f64
    } else {
        numbers[mid]
    }

}

fn get_mean_read_quality(numbers: &Vec<f64>) -> f64 {

    // Compute the mean of a vector of double-precision floats

    let sum: f64 = numbers.iter().sum();

    sum as f64 / numbers.len() as f64

}