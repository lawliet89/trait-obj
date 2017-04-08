# Trait Objects

This is in response to a question I
[posted](https://users.rust-lang.org/t/boxing-a-generic-trait-object-with-trait-type-parameters/10281) on Rust Users.

You can just run it with `cargo run`.

## Original Attempt

I wanted to implement the following, but was not successful in getting it to compile.

```rust
use std::fmt::Debug;
use std::io::Read;
use std::iter::Iterator;
use std::marker::PhantomData;

use csv;
use rustc_serialize::{Decodable, Encodable};

pub trait Validator {
    fn is_valid(&self) -> Result<(), String>;
}

/// Implement this trait for a single record
pub trait RecordValidator {
    fn is_valid(&self) -> Result<(), String>;
}

/// Implement this trait for a group of records
/// R is the type of record, Rdr is the type of the Reader
pub trait RecordsValidator<Rdr: Read> {
    type Record: RecordValidator + Decodable + Debug;

    fn csv_reader(&mut self) -> &mut csv::Reader<Rdr>;

    fn records<'a>(&'a mut self) -> RecordsValidatorIterator<'a, Rdr, Self::Record> {
        RecordsValidatorIterator::new(self.csv_reader().decode())
    }
}

/// Iterator over validated records
pub struct RecordsValidatorIterator<'a, Rdr, R>
    where Rdr: Read + 'a,
          R: RecordValidator + Decodable + Debug
{
    records: csv::DecodedRecords<'a, Rdr, R>,
    index: usize,
}

impl<'a, Rdr, R> RecordsValidatorIterator<'a, Rdr, R>
    where Rdr: Read + 'a,
          R: RecordValidator + Decodable + Debug
{
    fn new(records: csv::DecodedRecords<'a, Rdr, R>) -> Self {
        RecordsValidatorIterator {
            records: records,
            index: 0,
        }
    }

    fn validate_record(index: usize, record: Result<R, csv::Error>) -> Result<R, String> {
        match record {
            Ok(r) => {
                match r.is_valid() {
                    Ok(()) => Ok(r),
                    Err(err) => Err(format!("[validation.row.invalid] {}, {:?}, {:?}\n", index, err, r)),
                }
            }
            Err(err) => Err(format!("[decoding.row.invalid] {}, {:?}\n", index, err)),
        }
    }
}

impl<'a, Rdr, R> Iterator for RecordsValidatorIterator<'a, Rdr, R>
    where Rdr: Read + 'a,
          R: RecordValidator + Decodable + Debug
{
    type Item = Result<R, String>;

    fn next(&mut self) -> Option<Self::Item> {
        let record = self.records
            .next()
            .and_then(|r| Some(Self::validate_record(self.index, r)));
        self.index = self.index + 1;
        record
    }
}

/// A RecordsValidator for CSV files with comma seperator
pub struct CsvValidator<R: RecordValidator + Decodable + Debug, Rdr: Read> {
    reader: csv::Reader<Rdr>,
    _marker: PhantomData<R>,
}

impl<R: RecordValidator + Decodable + Debug, Rdr: Read> CsvValidator<R, Rdr> {
    pub fn new(reader: Rdr) -> Self {
        Self {
            reader: csv::Reader::from_reader(reader),
            _marker: Default::default(),
        }
    }
}

impl<R: RecordValidator + Decodable + Debug, Rdr: Read> RecordsValidator<Rdr> for CsvValidator<R, Rdr> {
    type Record = R;

    fn csv_reader(&mut self) -> &mut csv::Reader<Rdr> {
        &mut self.reader
    }
}

/// A RecordsValidator for PSV files with pipe seperator
pub struct PsvValidator<R: RecordValidator + Decodable + Debug, Rdr: Read> {
    reader: csv::Reader<Rdr>,
    _marker: PhantomData<R>,
}

impl<R: RecordValidator + Decodable + Debug, Rdr: Read> PsvValidator<R, Rdr> {
    pub fn new(reader: Rdr) -> Self {
        Self {
            reader: csv::Reader::from_reader(reader).delimiter(b'|'),
            _marker: Default::default(),
        }
    }
}

impl<R: RecordValidator + Decodable + Debug, Rdr: Read> RecordsValidator<Rdr> for PsvValidator<R, Rdr> {
    type Record = R;

    fn csv_reader(&mut self) -> &mut csv::Reader<Rdr> {
        &mut self.reader
    }
}
```

```rust
    let validator: Box<RecordsValidator<Read, Record=Record>> = Box::new(/* Trait Implementor*/);
```
