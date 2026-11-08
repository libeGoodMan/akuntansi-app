#import "_layout.typ": *
#let data = json("data.json")
#show: page-setup.with(data)
#kop(data)
#statement(data)

#v(8pt)
#text(size: 9pt, fill: if data.balanced { rgb("#1b7a43") } else { rgb("#b3261e") })[#data.note]
