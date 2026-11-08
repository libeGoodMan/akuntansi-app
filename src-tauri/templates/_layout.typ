// Tata letak bersama semua laporan: halaman A4, kop perusahaan, footer bernomor halaman,
// dan tabel laporan keuangan. Data (teks sudah diformat) datang dari Rust lewat data.json.

#let muted = luma(105)
#let hair = 0.5pt + luma(150)

#let page-setup(data, body) = {
  set document(title: data.title, author: data.company.name)
  set text(font: "Libertinus Serif", size: 10pt, lang: "id", features: ("tnum",))
  set page(
    paper: "a4",
    margin: (top: 1.8cm, bottom: 2cm, x: 1.8cm),
    footer: context {
      set text(size: 8pt, fill: muted)
      [Dicetak #data.printed_at #h(1fr) Halaman #counter(page).display() dari #counter(page).final().first()]
    },
  )
  body
}

// Kop: nama + alamat perusahaan, garis, judul dan periode laporan.
#let kop(data) = {
  align(center)[
    #text(size: 15pt, weight: "bold")[#data.company.name]
    #if data.company.address != none [
      \ #text(size: 9pt, fill: muted)[#data.company.address]
    ]
  ]
  v(4pt)
  line(length: 100%, stroke: 0.8pt)
  v(6pt)
  align(center)[
    #text(size: 13pt, weight: "bold")[#data.title] \
    #text(size: 10pt)[#data.period]
    #if "period2" in data and data.period2 != none [ \ #text(size: 9pt, fill: muted)[#data.period2] ]
  ]
  v(10pt)
}

#let bold(x) = text(weight: "bold", x)
#let rt(x) = align(right, x)

// Tabel laporan keuangan (Laba Rugi dan Neraca) dari daftar baris datar buatan Rust.
#let statement(data) = {
  let cmp = data.has_compare
  let ncols = if cmp { 5 } else { 3 }
  let cols = if cmp { (20mm, 1fr, 31mm, 31mm, 29mm) } else { (22mm, 1fr, 40mm) }

  let cells = ()
  cells.push(table.header(
    text(size: 9pt, fill: muted)[Kode],
    text(size: 9pt, fill: muted)[Keterangan],
    rt(bold(data.current_label)),
    ..if cmp { (rt(bold(data.compare_label)), rt(bold("Selisih"))) } else { () },
  ))
  cells.push(table.hline(stroke: 0.8pt))

  for l in data.lines {
    let indent = (l.level - 1) * 4mm
    if l.kind == "heading" {
      cells.push(table.cell(
        colspan: ncols,
        fill: if l.level == 1 { luma(232) } else { none },
        inset: (x: 4pt, y: 4.5pt),
      )[#pad(left: indent)[#bold(l.label)]])
    } else if l.kind == "account" {
      cells.push(text(size: 8.5pt, fill: muted)[#if l.code != none [#l.code]])
      cells.push(pad(left: indent)[#l.label])
      cells.push(rt[#l.amount])
      if cmp {
        cells.push(rt[#l.compare])
        cells.push(rt[#l.change])
      }
    } else {
      // subtotal, total, net
      let net = l.kind == "net"
      let fill = if net { luma(220) } else { none }
      cells.push(table.hline(stroke: if net { 0.8pt } else { hair }))
      cells.push(table.cell(fill: fill)[])
      cells.push(table.cell(fill: fill)[#pad(left: indent)[#bold(l.label)]])
      cells.push(table.cell(fill: fill)[#rt(bold(l.amount))])
      if cmp {
        cells.push(table.cell(fill: fill)[#rt(bold(l.compare))])
        cells.push(table.cell(fill: fill)[#rt(bold(l.change))])
      }
      if net { cells.push(table.hline(stroke: 0.8pt)) }
    }
  }

  table(
    columns: cols,
    stroke: none,
    inset: (x: 4pt, y: 3.5pt),
    align: (left, left, right, right, right),
    ..cells,
  )
}
