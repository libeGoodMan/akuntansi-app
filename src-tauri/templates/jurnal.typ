#import "_layout.typ": *
#let data = json("data.json")
#show: page-setup.with(data)
#kop(data)

#grid(
  columns: (28mm, 1fr, 28mm, 45mm),
  column-gutter: 8pt,
  row-gutter: 6pt,
  text(fill: muted)[Nomor], bold(data.number),
  text(fill: muted)[Tanggal], data.date,
  text(fill: muted)[Keterangan], data.description,
  text(fill: muted)[Status], data.status,
  text(fill: muted)[Referensi], if data.reference != none { data.reference } else { [-] },
  text(fill: muted)[Jenis], data.kind,
)
#v(10pt)

#let cells = ()
#cells.push(table.header(
  text(size: 9pt, fill: muted)[Kode],
  text(size: 9pt, fill: muted)[Akun / catatan],
  rt(bold[Debit]),
  rt(bold[Kredit]),
))
#cells.push(table.hline(stroke: 0.8pt))
#for l in data.lines {
  cells.push(text(size: 8.5pt, fill: muted)[#l.code])
  cells.push([#l.account #if l.memo != none [ \ #text(size: 8.5pt, fill: muted)[#l.memo]]])
  cells.push(rt[#l.debit])
  cells.push(rt[#l.credit])
}
#cells.push(table.hline(stroke: 0.8pt))
#cells.push(table.cell(colspan: 2, fill: luma(220))[#bold[Total]])
#cells.push(table.cell(fill: luma(220))[#rt(bold(data.total_debit))])
#cells.push(table.cell(fill: luma(220))[#rt(bold(data.total_credit))])
#cells.push(table.hline(stroke: 0.8pt))

#table(
  columns: (22mm, 1fr, 38mm, 38mm),
  stroke: none,
  inset: (x: 4pt, y: 4pt),
  align: (left, left, right, right),
  ..cells,
)

#v(1fr)

// Kotak tanda tangan
#grid(
  columns: (1fr, 1fr, 1fr),
  column-gutter: 14pt,
  ..for label in ("Dibuat oleh", "Diperiksa oleh", "Disetujui oleh") {
    (
      block(width: 100%, stroke: hair, inset: 8pt, height: 3.2cm)[
        #text(size: 9pt, fill: muted)[#label]
        #v(1fr)
        #line(length: 100%, stroke: 0.4pt)
        #text(size: 8pt, fill: muted)[Nama / tanggal]
      ],
    )
  },
)
