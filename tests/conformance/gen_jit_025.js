function f25(a, b) { let v0 = 3; for (let i = 0; i < a; i = i + 1) { v0 = b + 2; v0 = a - a; if (v0 > 62020) v0 = v0 % 636; v0 = a * a; v0 = 6 * a; v0 = v0 * 3; if (v0 > 562) v0 = v0 % 744; } return v0; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f25(k % 7, k);
console.log(acc);
console.log(f25(5, 3));
console.log(f25(20, 2147483));
console.log(f25(3, 0.5));
console.log(f25(4, -7.25));
console.log(f25(12, 1000000.0));
console.log(f25(0, 9));
console.log(f25(30, 65536));
