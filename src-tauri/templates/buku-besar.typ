#import "_layout.typ": *
#let data = json("data.json")
#show: page-setup.with(data)
#kop(data)

#grid(
  columns: (auto, 1fr),
  column-gutter: 10pt,
  row-gutter: 5pt,
  text(fill: muted)[Akun], bold(data.account),
  text(fill: muted)[Saldo normal], data.normal,
)
#v(6pt)

#let head(label) = rt(bold(label))
#let cells = ()
#cells.push(table.header(
  text(size: 9pt, fill: muted)[Tanggal],
  text(size: 9pt, fill: muted)[Nomor],
  text(size: 9pt, fill: muted)[Keterangan],
  head[Debit],
  head[Kredit],
  head[Saldo],
))
#cells.push(table.hline(stroke: 0.8pt))

// Baris saldo awal
#cells.push(table.cell(colspan: 5, fill: luma(240))[#text(style: "italic")[Saldo awal]])
#cells.push(table.cell(fill: luma(240))[#rt(data.opening)])

#for r in data.rows {
  cells.push(text(size: 8.5pt)[#r.date])
  cells.push(text(size: 8.5pt, fill: muted)[#r.number])
  cells.push([#r.description #if r.memo != none [ \ #text(size: 8.5pt, fill: muted)[#r.memo]]])
  cells.push(rt[#r.debit])
  cells.push(rt[#r.credit])
  cells.push(rt[#r.balance])
}

#cells.push(table.hline(stroke: 0.8pt))
#cells.push(table.cell(colspan: 3, fill: luma(220))[#bold[Total mutasi]])
#cells.push(table.cell(fill: luma(220))[#rt(bold(data.total_debit))])
#cells.push(table.cell(fill: luma(220))[#rt(bold(data.total_credit))])
#cells.push(table.cell(fill: luma(220))[#rt(bold(data.closing))])
#cells.push(table.hline(stroke: 0.8pt))

#set table.cell(breakable: false)
#table(
  columns: (19mm, 28mm, 1fr, 25mm, 25mm, 28mm),
  stroke: none,
  inset: (x: 4pt, y: 3.5pt),
  align: (left, left, left, right, right, right),
  ..cells,
)

#v(6pt)
#text(size: 9pt, fill: muted)[Kolom Saldo pada baris Total mutasi adalah saldo akhir.]
