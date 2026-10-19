console.log(1 + "2", "3" * "4", "10" - 2, true + 1, null + 1, undefined + 1, [] + [], [1] + [2]);
console.log(1 == "1", 0 == false, null == undefined, null == 0, NaN == NaN, "" == 0, [1] == 1);
console.log(1 === 1, "a" === "a", {} === {}, null === undefined);
console.log(typeof 1, typeof "s", typeof true, typeof undefined, typeof null, typeof {}, typeof [], typeof (() => 1), typeof notDefined);
console.log(!0, !!"", !!"0", 0 || "def", 1 && 2, null ?? "nn", 0 ?? "zero", void 0);
let x = 5; x += 2; x *= 3; x -= 1; x /= 4; x **= 2; x %= 7; console.log(x);
let y = null; y ??= 3; let z = 0; z ||= 9; let w = 1; w &&= 7; console.log(y, z, w);
let i = 0; console.log(i++, i, ++i, i--, i, --i);
console.log(3 > 2 > 1, 1 < 2 < 3, "10" < "9", 10 < 9, "a" < 1);
