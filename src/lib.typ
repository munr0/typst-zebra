#import "generic.typ": _generic

/// - data (str, bytes):
/// - options (dictionary):
/// - quiet-zone (int, bool, none):
/// - width (auto, length):
/// - height (auto, length):
/// - module-size (auto, length):
/// - fill (color, gradient):
/// - background-fill (none, color, gradient):
/// - clear (dictionary, array):
/// -> content
#let datamatrix(
  data,
  options: (:),
  quiet-zone: 0,
  width: auto,
  height: auto,
  module-size: auto,
  fill: black,
  background-fill: none,
  clear: (),
) = _generic(
  data,
  "datamatrix",
  options: options,
  quiet-zone: quiet-zone,
  width: width,
  height: height,
  module-size: module-size,
  fill: fill,
  background-fill: background-fill,
  clear: clear,
)

/// - data (str, bytes):
/// - options (dictionary):
/// - quiet-zone (int, bool, none):
/// - width (auto, length):
/// - height (auto, length):
/// - module-size (auto, length):
/// - fill (color, gradient):
/// - background-fill (none, color, gradient):
/// - clear (dictionary, array):
/// -> content
#let qrcode(
  data,
  options: (:),
  quiet-zone: 0,
  width: auto,
  height: auto,
  module-size: auto,
  fill: black,
  background-fill: none,
  clear: (),
) = _generic(
  data,
  "qrcode",
  options: options,
  quiet-zone: quiet-zone,
  width: width,
  height: height,
  module-size: module-size,
  fill: fill,
  background-fill: background-fill,
  clear: clear,
  default-quiet-zone: 4,
)
