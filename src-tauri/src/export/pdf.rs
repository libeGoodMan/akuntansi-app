//! Render PDF dari template Typst yang di-embed ke binary (tanpa WebView, tanpa font sistem).
//! Data laporan dikirim sebagai `data.json` virtual; template hanya mengatur tata letak.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde_json::Value;
use typst::diag::{FileError, FileResult, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime, Duration};
use typst_layout::PagedDocument;
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_pdf::PdfOptions;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Template {
    TrialBalance,
    IncomeStatement,
    BalanceSheet,
    Ledger,
    Voucher,
}

impl Template {
    fn source(self) -> &'static str {
        match self {
            Template::TrialBalance => include_str!("../../templates/neraca-saldo.typ"),
            Template::IncomeStatement => include_str!("../../templates/laba-rugi.typ"),
            Template::BalanceSheet => include_str!("../../templates/neraca.typ"),
            Template::Ledger => include_str!("../../templates/buku-besar.typ"),
            Template::Voucher => include_str!("../../templates/jurnal.typ"),
        }
    }
}

const LAYOUT: &str = include_str!("../../templates/_layout.typ");

struct Fonts {
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
}

/// Font yang ikut dalam binary (typst-assets): hasil PDF sama di semua PC.
static FONTS: LazyLock<Fonts> = LazyLock::new(|| {
    let fonts: Vec<Font> = typst_assets::fonts().flat_map(|data| Font::iter(Bytes::new(data))).collect();
    Fonts { book: LazyHash::new(FontBook::from_fonts(&fonts)), fonts }
});

static LIBRARY: LazyLock<LazyHash<Library>> = LazyLock::new(|| LazyHash::new(Library::default()));

fn file_id(path: &str) -> FileId {
    RootedPath::new(VirtualRoot::Project, VirtualPath::new(path).expect("path tetap valid")).intern()
}

struct ReportWorld {
    main: FileId,
    sources: HashMap<FileId, Source>,
    files: HashMap<FileId, Bytes>,
}

impl ReportWorld {
    fn new(template: Template, data: &Value) -> AppResult<Self> {
        let main = file_id("/main.typ");
        let layout = file_id("/_layout.typ");
        let json = serde_json::to_string(data).map_err(|e| AppError::Other(format!("Data laporan tidak valid: {e}")))?;

        let mut sources = HashMap::new();
        sources.insert(main, Source::new(main, template.source().to_string()));
        sources.insert(layout, Source::new(layout, LAYOUT.to_string()));

        let mut files = HashMap::new();
        files.insert(file_id("/data.json"), Bytes::from_string(json));
        Ok(Self { main, sources, files })
    }
}

impl World for ReportWorld {
    fn library(&self) -> &LazyHash<Library> {
        &LIBRARY
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &FONTS.book
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.sources.get(&id).cloned().ok_or_else(|| not_found(id))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if let Some(b) = self.files.get(&id) {
            return Ok(b.clone());
        }
        self.sources.get(&id).map(|s| Bytes::from_string(s.text().to_string())).ok_or_else(|| not_found(id))
    }

    fn font(&self, index: usize) -> Option<Font> {
        FONTS.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        use chrono::{Datelike, Local};
        let now = Local::now();
        Datetime::from_ymd(now.year(), now.month() as u8, now.day() as u8)
    }
}

fn not_found(id: FileId) -> FileError {
    FileError::NotFound(id.vpath().get_without_slash().into())
}

fn describe(errors: &[SourceDiagnostic]) -> String {
    errors.iter().map(|e| e.message.to_string()).collect::<Vec<_>>().join("; ")
}

pub(crate) fn compile(template: Template, data: &Value) -> AppResult<PagedDocument> {
    let world = ReportWorld::new(template, data)?;
    typst::compile::<PagedDocument>(&world)
        .output
        .map_err(|e| AppError::Other(format!("Template PDF gagal dirender: {}", describe(&e))))
}

/// Render satu laporan menjadi bytes PDF.
pub fn render(template: Template, data: &Value) -> AppResult<Vec<u8>> {
    let doc = compile(template, data)?;
    typst_pdf::pdf(&doc, &PdfOptions::default())
        .map_err(|e| AppError::Other(format!("Gagal membuat PDF: {}", describe(&e))))
}
