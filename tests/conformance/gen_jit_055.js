function f55(a, b) { let v0 = 4; for (let i = 0; i <= a; i = i + 1) { v0 = b + a; if (v0 > 20523) v0 = v0 % 879; v0 = 5 + 2; if (v0 > 86709) v0 = v0 % 484; v0 = 7 * b; } return v0; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f55(k % 7, k);
console.log(acc);
console.log(f55(5, 3));
console.log(f55(20, 2147483));
console.log(f55(3, 0.5));
console.log(f55(4, -7.25));
console.log(f55(12, 1000000.0));
console.log(f55(0, 9));
console.log(f55(30, 65536));
