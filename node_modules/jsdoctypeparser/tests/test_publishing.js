'use strict';

const {expect} = require('chai');

const {parse} = require('../lib/parsing.js');
const NodeType = require('../lib/NodeType.js');
const Publishing = require('../lib/publishing.js');

const {publish, createDefaultPublisher} = Publishing;

describe('publish', function() {
  describe('Publishers', function () {
    it('should have a default publisher for each node type', function() {
      const publisher = createDefaultPublisher();
      expect(Object.getOwnPropertyNames(publisher)).to.include.members(
        Object.getOwnPropertyNames(NodeType).map(p => NodeType[p])
      );
    });

    it('should can take a custom publisher by the 2nd argument', function() {
      const ast = {
        type: 'NAME',
        name: 'MyClass',
      };

      const customPublisher = createDefaultPublisher();
      customPublisher.NAME = function(node) {
        return '<a href="./types/' + node.name + '.html">' + node.name + '</a>';
      };

      const string = publish(ast, customPublisher);
      expect(string).to.equal('<a href="./types/MyClass.html">MyClass</a>');
    });
  });

  describe('Primitive types', function () {
    it('should return an undefined type with an "undefined" keyword', function() {
      const node = parse('undefined');
      expect(publish(node)).to.equal('undefined');
    });

    it('should return a null type with an "null" keyword', function() {
      const node = parse('null');
      expect(publish(node)).to.equal('null');
    });

    it('should return a primitive (boolean) type name', function() {
      const node = parse('boolean');
      expect(publish(node)).to.equal('boolean');
    });

    it('should return a string value type', function() {
      const node = parse('"stringValue"');
      expect(publish(node)).to.equal('"stringValue"');
    });

    it('should return a string value type (single quotes)', function() {
      const node = parse("'stringValue'");
      expect(publish(node)).to.equal("'stringValue'");
    });

    it('should return a string value type with escaped quote', function() {
      const node = parse('"string \\"Value"');
      expect(publish(node)).to.equal('"string \\"Value"');
    });

    it('should return a string value type with single extra backslash for odd count backslash series not before a quote', function() {
      const node = parse('"\\Odd count backslash sequence not before a quote\\add\\\\\\one backslash\\."');
      expect(publish(node)).to.equal('"\\\\Odd count backslash sequence not before a quote\\\\add\\\\\\\\one backslash\\\\."');
    });

    it('should return a string value type without adding backslashes for escaped backslash sequences (i.e., even count)', function() {
      const node = parse('"\\\\Even count (escaped)\\\\\\\\backslash sequences\\\\remain\\\\"');
      expect(publish(node)).to.equal('"\\\\Even count (escaped)\\\\\\\\backslash sequences\\\\remain\\\\"');
    });


    it('should return a number value type', function() {
      const node = parse('0123456789');
      expect(publish(node)).to.equal('0123456789');
    });


    it('should return a bin number value type', function() {
      const node = parse('0b01');
      expect(publish(node)).to.equal('0b01');
    });


    it('should return an oct number value type', function() {
      const node = parse('0o01234567');
      expect(publish(node)).to.equal('0o01234567');
    });


    it('should return a hex number value type', function() {
      const node = parse('0x0123456789abcdef');
      expect(publish(node)).to.equal('0x0123456789abcdef');
    });
  });

  describe('Wildcard types', function () {
    it('should return an all type', function() {
      const node = parse('*');
      expect(publish(node)).to.equal('*');
    });

    it('should return an unknown type', function() {
      const node = parse('?');
      expect(publish(node)).to.equal('?');
    });
  });

  describe('Generics', function () {
    it('should return a generic type with a parameter', function() {
      const node = parse('Array.<string>');
      expect(publish(node)).to.equal('Array.<string>');
    });


    it('should return a generic type with 2 parameters', function() {
      const node = parse('Object.<string, number>');
      expect(publish(node)).to.equal('Object.<string, number>');
    });


    it('should return a JsDoc-formal generic type', function() {
      const node = parse('String[]');
      expect(publish(node)).to.equal('String[]');
    });
  });

  describe('Record types', function () {
    it('should return a record type with an entry', function() {
      const node = parse('{myNum}');
      expect(publish(node)).to.equal('{myNum}');
    });


    it('should return a record type with 2 entries', function() {
      const node = parse('{myNum: number, myObject}');
      expect(publish(node)).to.equal('{myNum: number, myObject}');
    });

    it('should return a quoted record type key', function() {
      const node = parse('{"myNum": number, "myObject"}');
      expect(publish(node)).to.equal('{"myNum": number, "myObject"}');
    });

    it('should return a quoted record type key (single quotes)', function() {
      const node = parse("{'myNum': number, 'myObject'}");
      expect(publish(node)).to.equal("{'myNum': number, 'myObject'}");
    });

    it('should return a quoted record type key with escaped quote', function() {
      const node = parse('{"my\\"Num": number, "myObject"}');
      expect(publish(node)).to.equal('{"my\\"Num": number, "myObject"}');
    });

    it('should return a quoted record type key with single extra backslash for odd count backslash series not before a quote', function() {
      const node = parse('{"\\Odd count backslash sequence not before a quote\\add\\\\\\one backslash\\.": number, "myObject"}');
      expect(publish(node)).to.equal('{"\\\\Odd count backslash sequence not before a quote\\\\add\\\\\\\\one backslash\\\\.": number, "myObject"}');
    });

    it('should return a quoted record type key without adding backslashes for escaped backslash sequences (i.e., even count)', function() {
      const node = parse('{"\\\\Even count (escaped)\\\\\\\\backslash sequences\\\\remain\\\\": number, "myObject"}');
      expect(publish(node)).to.equal('{"\\\\Even count (escaped)\\\\\\\\backslash sequences\\\\remain\\\\": number, "myObject"}');
    });

    it('should return an optional record type by type', function() {
      const node = parse('{myNum: number=}');
      expect(publish(node)).to.equal('{myNum: number=}');
    });

    it('should return an optional record type by key', function() {
      const node = parse('{myNum?: number}');
      expect(publish(node)).to.equal('{myNum?: number}');
    });
  });

  describe('Tuple types', function () {
    it('should return a tuple type', function() {
      const node = parse('[]');
      expect(publish(node)).to.equal('[]');
    });


    it('should return a tuple type with an entry', function() {
      const node = parse('[number]');
      expect(publish(node)).to.equal('[number]');
    });


    it('should return a tuple type with 2 entries', function() {
      const node = parse('[number, MyObject]');
      expect(publish(node)).to.equal('[number, MyObject]');
    });


    it('should return a generic type with a parameter as a record type', function() {
      const node = parse('Array<{length}>');
      expect(publish(node)).to.equal('Array<{length}>');
    });


    it('should return a generic type with a parameter as a tuple type', function() {
      const node = parse('Array<[string, number]>');
      expect(publish(node)).to.equal('Array<[string, number]>');
    });
  });

  describe('Function/Constructor/Arrow types', function () {
    it('should return a function type', function() {
      const node = parse('Function');
      expect(publish(node)).to.equal('Function');
    });


    it('should return a function type with no parameters', function() {
      const node = parse('function()');
      expect(publish(node)).to.equal('function()');
    });

    it('should return a function type with variadic', function() {
      const node = parse('function(...a)');
      expect(publish(node)).to.equal('function(...a)');
    });

    it('should return a function type with variadic and no operand', function() {
      const node = parse('function(...)');
      expect(publish(node)).to.equal('function(...)');
    });


    it('should return a function type with a parameter', function() {
      const node = parse('function(string)');
      expect(publish(node)).to.equal('function(string)');
    });


    it('should return a function type with 2 parameters', function() {
      const node = parse('function(string, boolean)');
      expect(publish(node)).to.equal('function(string, boolean)');
    });


    it('should return a function type with a return', function() {
      const node = parse('function(): number');
      expect(publish(node)).to.equal('function(): number');
    });


    it('should return a function type with a context', function() {
      const node = parse('function(this:goog.ui.Menu, string)');
      expect(publish(node)).to.equal('function(this: goog.ui.Menu, string)');
    });


    it('should return a constructor type', function() {
      const node = parse('function(new:goog.ui.Menu, string)');
      expect(publish(node)).to.equal('function(new: goog.ui.Menu, string)');
    });


    it('should return a function type with a variable parameter', function() {
      const node = parse('function(string, ...number): number');
      expect(publish(node)).to.equal('function(string, ...number): number');
    });


    it('should return a function type having parameters with some type operators', function() {
      const node = parse('function(?string=, number=)');
      expect(publish(node)).to.equal('function(?string=, number=)');
    });

    it('should return an arrow type with no parameters', function() {
      const node = parse('() => string');
      expect(publish(node)).to.equal('() => string');
    });

    it('should return an arrow type with two parameters', function() {
      const node = parse('(x: true, y: false) => string');
      expect(publish(node)).to.equal('(x: true, y: false) => string');
    });

    it('should return an arrow type with one parameter', function() {
      const node = parse('(x: true) => string');
      expect(publish(node)).to.equal('(x: true) => string');
    });

    it('should return an arrow type with one variadic parameter', function() {
      const node = parse('(...x: any[]) => string');
      expect(publish(node)).to.equal('(...x: any[]) => string');
    });

    it('should return a construct signature with one parameter', function() {
      const node = parse('new (x: true) => string');
      expect(publish(node)).to.equal('new (x: true) => string');
    });

    it('should return a goog.ui.Component#forEachChild', function() {
      const node = parse('function(this:T,?,number):?');
      expect(publish(node)).to.equal('function(this: T, ?, number): ?');
    });
  });

  describe('BroadNamepathExpr types', function () {
    describe('Single component', function () {
      it('should return a global type name', function() {
        const node = parse('Window');
        expect(publish(node)).to.equal('Window');
      });
    });

    describe('Multipart', function () {
      it('should return a user-defined type name', function() {
        const node = parse('goog.ui.Menu');
        expect(publish(node)).to.equal('goog.ui.Menu');
      });

      it('should return a quoted `MemberName` type', function() {
        const node = parse('namespace."memberNameValue"');
        expect(publish(node)).to.equal('namespace."memberNameValue"');
      });

      it('should return a quoted `MemberName` type (single quotes)', function() {
        const node = parse("namespace.'memberNameValue'");
        expect(publish(node)).to.equal("namespace.'memberNameValue'");
      });

      it('should return a quoted `MemberName` type with escaped quote', function() {
        const node = parse('namespace."member name \\"Value"');
        expect(publish(node)).to.equal('namespace."member name \\"Value"');
      });

      it('should return a quoted `MemberName` type with single extra backslash for odd count backslash series not before a quote', function() {
        const node = parse('namespace."\\Odd count backslash sequence not before a quote\\add\\\\\\one backslash\\."');
        expect(publish(node)).to.equal('namespace."\\\\Odd count backslash sequence not before a quote\\\\add\\\\\\\\one backslash\\\\."');
      });

      it('should return a quoted `MemberName` type without adding backslashes for escaped backslash sequences (i.e., even count)', function() {
        const node = parse('namespace."\\\\Even count (escaped)\\\\\\\\backslash sequences\\\\remain\\\\"');
        expect(publish(node)).to.equal('namespace."\\\\Even count (escaped)\\\\\\\\backslash sequences\\\\remain\\\\"');
      });
    });

    describe('external', function () {
      it('should return an external node type', function() {
        const node = parse('external:string');
        expect(publish(node)).to.equal('external:string');
      });
      it('should return an external node type with an instance member method', function() {
        const node = parse('external : String#rot13');
        expect(publish(node)).to.equal('external:String#rot13');
      });
      it('should return a quoted external node type', function() {
        const node = parse('external:"jQuery.fn"');
        expect(publish(node)).to.equal('external:"jQuery.fn"');
      });
      it('should return a quoted external node type with a static member method and event', function() {
        const node = parse('external:"jQuery.fn".someMethod#event:abc');
        expect(publish(node)).to.equal('external:"jQuery.fn".someMethod#event:abc');
      });
    });


    describe('module', function () {
      it('should return a module type', function() {
        const node = parse('module:foo/bar');
        expect(publish(node)).to.equal('module:foo/bar');
      });

      it('should return a module type with member', function() {
        const node = parse('module:foo/bar#abc');
        expect(publish(node)).to.equal('module:foo/bar#abc');
      });

      it('should return a module type with event member', function() {
        const node = parse('module:foo/bar#event:abc');
        expect(publish(node)).to.equal('module:foo/bar#event:abc');
      });


      it('should return a module type with a prefix nullable type operator', function() {
        const node = parse('?module:foo/bar');
        expect(publish(node)).to.equal('?module:foo/bar');
      });


      it('should return a module type with a postfix nullable type operator', function() {
        const node = parse('module:foo/bar?');
        expect(publish(node)).to.equal('?module:foo/bar');
      });

      it('should return a module type with a generic type operator', function() {
        // Because the new generic type syntax was arrived, the old type generic
        // with the module keyword is not equivalent to the legacy behavior.
        //
        // For example, we get 2 parts as 'module:foo/bar.' and '<string>', when
        // the following type expression are arrived.
        //   const node = parse('module:foo/bar.<string>');
        const node = parse('module:foo/bar<string>');
        expect(publish(node)).to.equal('module:foo/bar<string>');
      });

      it('should return a quoted module type', function() {
        const node = parse('module:"module/path".member');
        expect(publish(node)).to.equal('module:"module/path".member');
      });

      it('should return a quoted module type (single quotes)', function() {
        const node = parse("module:'module/path'.member");
        expect(publish(node)).to.equal("module:'module/path'.member");
      });

      it('should return a quoted module type with escaped quote', function() {
        const node = parse('module:"module/path\\"Value".member');
        expect(publish(node)).to.equal('module:"module/path\\"Value".member');
      });

      it('should return a quoted module type with single extra backslash for odd count backslash series not before a quote', function() {
        const node = parse('module:"\\Odd/count/backslash/sequence/not/before/a/quote\\add\\\\\\one/backslash\\.".member');
        expect(publish(node)).to.equal('module:"\\\\Odd/count/backslash/sequence/not/before/a/quote\\\\add\\\\\\\\one/backslash\\\\.".member');
      });

      it('should return a quoted module type without adding backslashes for escaped backslash sequences (i.e., even count)', function() {
        const node = parse('module:"\\\\Even/count/(escaped)\\\\\\\\backslash/sequences\\\\remain\\\\".member');
        expect(publish(node)).to.equal('module:"\\\\Even/count/(escaped)\\\\\\\\backslash/sequences\\\\remain\\\\".member');
      });
    });
  });

  describe('Types with modifiers', function () {
    it('should return a variable type', function () {
      const node = parse('...number');
      expect(publish(node)).to.equal('...number');
    });


    it('should return an optional type with an optional type operator on the head', function() {
      const node = parse('=number');
      expect(publish(node)).to.equal('number=');
    });


    it('should return an optional type with an optional type operator on the tail', function() {
      const node = parse('number=');
      expect(publish(node)).to.equal('number=');
    });

    it('should return a nullable type with a nullable type operator on the head', function() {
      const node = parse('?number');
      expect(publish(node)).to.equal('?number');
    });


    it('should return a nullable type with a nullable type operator on the tail', function() {
      const node = parse('goog.ui.Component?');
      expect(publish(node)).to.equal('?goog.ui.Component');
    });


    it('should return a non-nullable type with a nullable type operator on the head', function() {
      const node = parse('!Object');
      expect(publish(node)).to.equal('!Object');
    });


    it('should return a non-nullable type with a nullable type operator on the tail', function() {
      const node = parse('Object!');
      expect(publish(node)).to.equal('!Object');
    });
  });

  describe('Type combinations', function () {
    it('should return a formal type union', function() {
      const node = parse('(number|boolean)');
      expect(publish(node)).to.equal('(number|boolean)');
    });


    it('should return a informal type union', function() {
      const node = parse('number|boolean');
      expect(publish(node)).to.equal('number|boolean');
    });
  });

  describe('Types with operations', function () {
    it('should return a type query node', function() {
      const node = parse('typeof x');
      expect(publish(node)).to.equal('typeof x');
    });

    it('should return a key query node', function() {
      const node = parse('keyof x');
      expect(publish(node)).to.equal('keyof x');
    })

    it('should return an import type node', function() {
      const node = parse('import("./lodash4ever")');
      expect(publish(node)).to.equal('import("./lodash4ever")');
    });
  });
});
