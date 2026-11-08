#import "_layout.typ": *
#let data = json("data.json")
#show: page-setup.with(data)
#kop(data)

#let cells = ()
#cells.push(table.header(
  text(size: 9pt, fill: muted)[Kode],
  text(size: 9pt, fill: muted)[Nama akun],
  rt(bold[Debit]),
  rt(bold[Kredit]),
))
#cells.push(table.hline(stroke: 0.8pt))

#for g in data.groups {
  cells.push(table.cell(colspan: 4, fill: luma(232), inset: (x: 4pt, y: 4.5pt))[#bold(g.name)])
  for r in g.rows {
    cells.push(text(size: 8.5pt, fill: muted)[#r.code])
    cells.push(r.name)
    cells.push(rt[#r.debit])
    cells.push(rt[#r.credit])
  }
}

#cells.push(table.hline(stroke: 0.8pt))
#cells.push(table.cell(colspan: 2, fill: luma(220))[#bold[Total]])
#cells.push(table.cell(fill: luma(220))[#rt(bold(data.total_debit))])
#cells.push(table.cell(fill: luma(220))[#rt(bold(data.total_credit))])
#cells.push(table.hline(stroke: 0.8pt))

#table(
  columns: (22mm, 1fr, 38mm, 38mm),
  stroke: none,
  inset: (x: 4pt, y: 3.5pt),
  align: (left, left, right, right),
  ..cells,
)

#v(8pt)
#text(size: 9pt, fill: if data.balanced { rgb("#1b7a43") } else { rgb("#b3261e") })[#data.note]
