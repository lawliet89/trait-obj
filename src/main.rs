extern crate csv;
extern crate rustc_serialize;

use std::io::{Cursor, Read};
use std::iter::Iterator;
use std::marker::PhantomData;
use std::ops::Deref;

use rustc_serialize::{Decodable, Encodable};

/// Implement this trait for a "Record" i.e. line from a CSV/PSV file
pub trait Record: Encodable + Decodable + 'static {
    fn is_valid(&self) -> Result<(), String>;
}

/// Implementation detail trait -- this trait's job is to deserialize a bunch of string bytes into a `Record`,
/// and then call `Record::is_valid`
trait Validator {
    /// Implementors should first deserialze into a `Record` and call `Record::is_valid`
    fn validate(&self, record: &Vec<csv::ByteString>) -> Result<(), String>;
}

/// Implementor of `Validator`
struct ValidatorHusk<R: Record> {
    _marker: PhantomData<R>,
}

impl<R: Record> Validator for ValidatorHusk<R> {
    fn validate(&self, record: &Vec<csv::ByteString>) -> Result<(), String> {
        let mut record = csv::Decoded::new(record.clone());
        let record: R = Decodable::decode(&mut record)
            .map_err(|s| s.to_string())?;
        record.is_valid()
    }
}

impl<R: Record> ValidatorHusk<R> {
    /// Create a `Records` for a record type `R` and `Rdr: Read`
    fn records<Rdr: Read>(reader: Rdr, delimiter: u8) -> Records<Rdr> {
        let validator = Self { _marker: Default::default() };
        Records::new(csv::Reader::from_reader(reader).delimiter(delimiter),
                     Box::new(validator))
    }
}

/// Struct to hold a `csv::Reader` and an associated `Validator`. Use `ValidatorHusk::records` to construct a new
/// instance
pub struct Records<Rdr: Read> {
    reader: csv::Reader<Rdr>,
    validator: Box<Validator>,
}

impl<Rdr: Read> Records<Rdr> {
    fn new(reader: csv::Reader<Rdr>, validator: Box<Validator>) -> Self {
        Self {
            reader: reader,
            validator: validator,
        }
    }

    fn byte_strings_to_string(record: Vec<csv::ByteString>) -> String {
        let strings: Vec<String> = record
            .into_iter()
            .map(|s| {
                     // should always be successful
                     String::from_utf8(s).unwrap()
                 })
            .collect();
        strings.join(", ")
    }

    pub fn records<'a>(&'a mut self) -> RecordsIterator<'a, Rdr> {
        RecordsIterator::new(self.reader.byte_records(), Box::new(self.validator.deref()))
    }
}

/// Iterator for validated Records
pub struct RecordsIterator<'a, Rdr: Read + 'a> {
    records: csv::ByteRecords<'a, Rdr>,
    validator: Box<&'a Validator>,
    index: usize,
}

impl<'a, Rdr: Read + 'a> RecordsIterator<'a, Rdr> {
    fn new(records: csv::ByteRecords<'a, Rdr>, validator: Box<&'a Validator>) -> Self {
        Self {
            records: records,
            validator: validator,
            index: 0,
        }
    }

    fn validate_record(&self,
                       record: Result<Vec<csv::ByteString>, csv::Error>)
                       -> Result<Vec<csv::ByteString>, String> {
        match record {
            Ok(r) => {
                match self.validator.validate(&r) {
                    Ok(()) => Ok(r),
                    Err(err) => {
                        Err(format!("[validation.row.invalid] {}, {:?}, {:?}\n",
                                    self.index,
                                    err,
                                    r))
                    }
                }
            }
            Err(err) => Err(format!("[decoding.row.invalid] {}, {:?}\n", self.index, err)),
        }
    }
}

impl<'a, Rdr: Read + 'a> Iterator for RecordsIterator<'a, Rdr> {
    type Item = Result<Vec<csv::ByteString>, String>;

    fn next(&mut self) -> Option<Self::Item> {
        let record = self.records
            .next()
            .and_then(|r| Some(self.validate_record(r)));
        self.index += 1;
        record
    }
}

#[derive(RustcDecodable, RustcEncodable, Debug)]
struct TestRecord {
    valid: bool,
    name: String,
}

impl Record for TestRecord {
    fn is_valid(&self) -> Result<(), String> {
        if self.valid {
            Ok(())
        }
        else {
            Err(format!("{} is not valid", self.name).to_string())
        }
    }
}

fn main() {
    let csv = r#"valid,name
true,foo
false,bar
true,baz"#;

    let reader = Cursor::new(csv.as_bytes());
    let mut validator = ValidatorHusk::<TestRecord>::records(reader, b',');

    for record in validator.records() {
        match record {
            Ok(record) => println!("Valid: {:?}", record),
            Err(e) => println!("{}", e)
        }
    }
}
