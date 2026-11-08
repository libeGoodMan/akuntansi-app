#import "_layout.typ": *
#let data = json("data.json")
#show: page-setup.with(data)
#kop(data)
#statement(data)
