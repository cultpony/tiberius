#[macro_export]
macro_rules! tantivy_raw_text_field {
    ($builder:ident, $name:ident) => {
        $builder.add_text_field(
            stringify!($name),
            TextOptions::default().set_stored().set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("raw")
                    .set_index_option(IndexRecordOption::WithFreqs),
            ),
        )
    };
}

#[macro_export]
macro_rules! tantivy_bool_text_field {
    ($builder:ident, $name:ident) => {
        $builder.add_text_field(
            stringify!($name),
            TextOptions::default().set_stored().set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("raw")
                    .set_index_option(IndexRecordOption::Basic),
            ),
        )
    };
}

#[macro_export]
macro_rules! tantivy_text_field {
    ($builder:ident, $name:ident) => {
        $builder.add_text_field(stringify!($name), TextOptions::default())
    };
}

#[macro_export]
macro_rules! tantivy_indexed_text_field {
    ($builder:ident, $name:ident) => {
        $builder.add_text_field(
            stringify!($name),
            TextOptions::default().set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("autocomplete")
                    .set_index_option(IndexRecordOption::Basic),
            ),
        )
    };
}

#[macro_export]
macro_rules! tantivy_u64_field {
    ($builder:ident, $name:ident) => {
        $builder.add_u64_field(
            stringify!($name),
            NumericOptions::default()
                .set_indexed()
                .set_stored()
                .set_fast(Cardinality::SingleValue),
        )
    };
}

#[macro_export]
macro_rules! tantivy_date_field {
    ($builder:ident, $name:ident) => {
        $builder.add_date_field(
            stringify!($name),
            DateOptions::default()
                .set_indexed()
                .set_stored()
                .set_fast(Cardinality::SingleValue),
        );
        $builder.add_u64_field(
            concat!(stringify!($name), "_ts"),
            NumericOptions::default()
                .set_indexed()
                .set_stored()
                .set_fast(Cardinality::SingleValue),
        )
    };
}

#[macro_export]
macro_rules! doc_add_ {
    ($doc:ident, $schema:ident, text, $name:ident, $value:expr) => {
        $doc.add_text($schema.get_field(stringify!($name)).unwrap(), $value)
    };
    ($doc:ident, $schema:ident, option<text>, $name:ident, $value:expr) => {
        if let Some(v) = $value {
            $doc.add_text($schema.get_field(stringify!($name)).unwrap(), v)
        }
    };
    ($doc:ident, $schema:ident, u64, $name:ident, $value:expr) => {
        $doc.add_u64($schema.get_field(stringify!($name)).unwrap(), $value)
    };
    ($doc:ident, $schema:ident, option<u64>, $name:ident, $value:expr) => {
        if let Some(v) = $value {
            $doc.add_u64($schema.get_field(stringify!($name)).unwrap(), v)
        }
    };
    ($doc:ident, $schema:ident, date, $name:ident, $value:expr) => {
        $doc.add_date($schema.get_field(stringify!($name)).unwrap(), $value)
    };
}
