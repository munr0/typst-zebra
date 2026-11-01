#import "../src/lib.typ": qrcode

#set page(
  width: auto,
  height: auto,
  margin: 0pt,
  fill: none,
)

#let qrcode = qrcode.with(module-size: 5pt, quiet-zone: true, background-fill: white)

#let logo = box(
  width: 100%,
  height: 100%,
  fill: black,
  radius: 20%,
  align(center + horizon, text(fill: white, weight: "bold", size: 7pt, "T")),
)

#stack(
  dir: ltr,
  spacing: 10pt,
  ..(
    // auto graphic-size: library picks a safe odd square from EC-H budget
    qrcode("https://typst.app", graphic: logo),
    // explicit graphic-size: caller controls the cleared area (30% of QR width)
    qrcode("https://typst.app", graphic: logo, graphic-size: 30%),
    // coloured fill with a circle graphic
    qrcode(
      "https://typst.app",
      fill: eastern,
      graphic: circle(width: 100%, height: 100%, fill: white),
    ),
  ).map(content => block(content, inset: 10pt, radius: 5pt)),
  qrcode(
    "MUNROA.COM",
    fill: eastern,
    graphic: circle(width: 100%, height: 100%, fill: white),
    graphic-size: 40%,
  ),
  qrcode(
    "https://" + lorem(100),
    fill: eastern,
    graphic: circle(width: 100%, height: 100%, fill: white),
  ),
)
