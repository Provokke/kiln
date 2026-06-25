function f41(a, b) { let v0 = 2; let v1 = 0; for (let i = 0; i <= a; i = i + 1) { v1 = 6 + v1; v1 = v0 + v0; v1 = v1 - v1; if (v1 > 74116) v1 = v1 % 196; v1 = 9 - v1; if (v1 > 94850) v1 = v1 % 128; v1 = b - v1; } return v0 + v1; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f41(k % 7, k);
console.log(acc);
console.log(f41(5, 3));
console.log(f41(20, 2147483));
console.log(f41(3, 0.5));
console.log(f41(4, -7.25));
console.log(f41(12, 1000000.0));
console.log(f41(0, 9));
console.log(f41(30, 65536));
