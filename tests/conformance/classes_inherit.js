class Animal { constructor(name) { this.name = name; } speak() { return this.name + " makes a sound"; } }
class Dog extends Animal { speak() { return super.speak() + " (woof)"; } }
class Puppy extends Dog { constructor(name) { super(name + " jr"); this.young = true; } }
const d = new Puppy("Rex");
console.log(d.speak(), d.young, d instanceof Animal, d instanceof Dog);
console.log(d);
console.log(Dog, Animal);
