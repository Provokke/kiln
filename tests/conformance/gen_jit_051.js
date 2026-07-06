function f51(a, b) { let v0 = 5; for (let i = 0; i !== a; i = i + 1) { v0 = 7 - v0; v0 = b * a; v0 = v0 - a; if (v0 > 30092) v0 = v0 % 578; v0 = a - b; if (v0 > 62599) v0 = v0 % 636; } return v0; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f51(k % 7, k);
console.log(acc);
console.log(f51(5, 3));
console.log(f51(20, 2147483));
console.log(f51(3, 0.5));
console.log(f51(4, -7.25));
console.log(f51(12, 1000000.0));
console.log(f51(0, 9));
console.log(f51(30, 65536));
