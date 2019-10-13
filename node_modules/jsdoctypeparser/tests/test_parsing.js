'use strict';

const {expect} = require('chai');

const NodeType = require('../lib/NodeType.js');
const Parser = require('../lib/parsing.js');
const SyntaxType = require('../lib/SyntaxType.js');

/** @typedef {{type: import('../lib/NodeType').Type}} Node */

const {
  GenericTypeSyntax, UnionTypeSyntax, VariadicTypeSyntax,
  OptionalTypeSyntax, NullableTypeSyntax, NotNullableTypeSyntax,
} = SyntaxType;

describe('Parser', function() {
  describe('Primitive types', function () {
    it('should return a number value type node when "0123456789" arrived', function() {
      const typeExprStr = '0123456789';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createNumberValueNode(typeExprStr);
      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a number value type node when "0.0" arrived', function() {
      const typeExprStr = '0.0';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createNumberValueNode(typeExprStr);
      expect(node).to.deep.equal(expectedNode);
    });

    it('should return a number value type node when ".0" arrived', function() {
      const typeExprStr = '.0';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createNumberValueNode(typeExprStr);
      expect(node).to.deep.equal(expectedNode);
    });

    it('should return a number value type node when "-0" arrived', function() {
      const typeExprStr = '-0';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createNumberValueNode(typeExprStr);
      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a number value type node when "+0" arrived', function() {
      const typeExprStr = '+0';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createNumberValueNode(typeExprStr);
      expect(node).to.deep.equal(expectedNode);
    });

    it('should return a number value type node when "3e5" arrived', function() {
      const typeExprStr = '3e5';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createNumberValueNode(typeExprStr);
      expect(node).to.deep.equal(expectedNode);
    });

    it('should return a number value type node when "3e+5" arrived', function() {
      const typeExprStr = '3e+5';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createNumberValueNode(typeExprStr);
      expect(node).to.deep.equal(expectedNode);
    });

    it('should return a number value type node when "3e-5" arrived', function() {
      const typeExprStr = '3e-5';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createNumberValueNode(typeExprStr);
      expect(node).to.deep.equal(expectedNode);
    });

    it('should return a number value type node when "3.14e5" arrived', function() {
      const typeExprStr = '3.14e5';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createNumberValueNode(typeExprStr);
      expect(node).to.deep.equal(expectedNode);
    });

    it('should return a number value type node when "0b01" arrived', function() {
      const typeExprStr = '0b01';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createNumberValueNode(typeExprStr);
      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a number value type node when "0o01234567" arrived', function() {
      const typeExprStr = '0o01234567';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createNumberValueNode(typeExprStr);
      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a number value type node when "0x0123456789abcdef" arrived', function() {
      const typeExprStr = '0x0123456789abcdef';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createNumberValueNode(typeExprStr);
      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a string value type node when \'""\' arrived', function() {
      const typeExprStr = '""';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createStringValueNode('');
      expect(node).to.deep.equal(expectedNode);
    });

    it('should return a string value type node when "\'\'" arrived', function() {
      const typeExprStr = "''";
      const node = Parser.parse(typeExprStr);

      const expectedNode = createStringValueNode('', 'single');
      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a string value type node when \'"string"\' arrived', function() {
      const typeExprStr = '"string"';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createStringValueNode('string');
      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a string value type node when \'"マルチバイト"\' arrived', function() {
      const typeExprStr = '"マルチバイト"';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createStringValueNode('マルチバイト');
      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a string value type node when \'"\\n\\"\\t"\' arrived', function() {
      const typeExprStr = '"\\n\\"\\t"';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createStringValueNode('\\n"\\t');
      expect(node).to.deep.equal(expectedNode);
    });

    it('should return a string value type node when \'"\\string with\\ \\"escapes\\\\"\' arrived', function() {
      const typeExprStr = '"\\string with\\ \\"escapes\\\\"';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createStringValueNode('\\string with\\ "escapes\\');
      expect(node).to.deep.equal(expectedNode);
    });

    it('should throw for string value type node with unescaped quotes', function() {
      const typeExprStr = '"string with "unescaped quote"';
      expect(function () {
        Parser.parse(typeExprStr);
      }).to.throw('or end of input but "u" found.');
    });

    it('should throw for string value type node with unescaped quotes (preceded by escaped backslash)', function() {
      const typeExprStr = '"string with \\\\"unescaped quote"';
      expect(function () {
        Parser.parse(typeExprStr);
      }).to.throw('or end of input but "u" found.');
    });


    it('should throw a syntax error when "" arrived', function() {
      const typeExprStr = '';

      expect(function() {
        Parser.parse(typeExprStr);
      }).to.throw(Parser.JSDocTypeSyntaxError);
    });

    it('should throw a syntax error when "Invalid type" arrived', function() {
      const typeExprStr = 'Invalid type';

      expect(function() {
        Parser.parse(typeExprStr);
      }).to.throw(Parser.JSDocTypeSyntaxError);
    });
  });

  describe('Wildcard types', function () {
    it('should return an any type node when "*" arrived', function() {
      const typeExprStr = '*';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createAnyTypeNode();
      expect(node).to.deep.equal(expectedNode);
    });


    it('should return an unknown type node when "?" arrived', function() {
      const typeExprStr = '?';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createUnknownTypeNode();
      expect(node).to.deep.equal(expectedNode);
    });
  });

  describe('Generic types', function () {
    it('should return a generic type node when "[][]" arrived', function() {
      const typeExprStr = '[][]';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createGenericTypeNode(
        createTypeNameNode('Array'),
        [
          createTupleTypeNode([]),
        ],
        GenericTypeSyntax.SQUARE_BRACKET);

      expect(node).to.deep.equal(expectedNode);
    });

    it('should return a generic type node when "[TupleType][]" arrived', function() {
      const typeExprStr = '[TupleType][]';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createGenericTypeNode(
        createTypeNameNode('Array'),
        [
          createTupleTypeNode([
            createTypeNameNode('TupleType'),
          ]),
        ],
        GenericTypeSyntax.SQUARE_BRACKET);

      expect(node).to.deep.equal(expectedNode);
    });

    it('should return a generic type node when "[TupleType1, TupleType2][]" arrived', function() {
      const typeExprStr = '[TupleType1, TupleType2][]';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createGenericTypeNode(
        createTypeNameNode('Array'),
        [
          createTupleTypeNode([
            createTypeNameNode('TupleType1'),
            createTypeNameNode('TupleType2'),
          ]),
        ],
        GenericTypeSyntax.SQUARE_BRACKET);

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a generic type node when "Generic<ParamType>" arrived', function() {
      const typeExprStr = 'Generic<ParamType>';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createGenericTypeNode(
        createTypeNameNode('Generic'), [
          createTypeNameNode('ParamType'),
      ], GenericTypeSyntax.ANGLE_BRACKET);

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a generic type node when "Generic<Inner<ParamType>>" arrived', function() {
      const typeExprStr = 'Generic<Inner<ParamType>>';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createGenericTypeNode(
        createTypeNameNode('Generic'), [
          createGenericTypeNode(
            createTypeNameNode('Inner'), [ createTypeNameNode('ParamType') ]
          ),
      ], GenericTypeSyntax.ANGLE_BRACKET);

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a generic type node when "Generic<ParamType1,ParamType2>"' +
       ' arrived', function() {
      const typeExprStr = 'Generic<ParamType1,ParamType2>';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createGenericTypeNode(
        createTypeNameNode('Generic'), [
          createTypeNameNode('ParamType1'),
          createTypeNameNode('ParamType2'),
        ], GenericTypeSyntax.ANGLE_BRACKET);

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a generic type node when "Generic < ParamType1 , ParamType2 >"' +
       ' arrived', function() {
      const typeExprStr = 'Generic < ParamType1, ParamType2 >';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createGenericTypeNode(
        createTypeNameNode('Generic'), [
          createTypeNameNode('ParamType1'),
          createTypeNameNode('ParamType2'),
        ], GenericTypeSyntax.ANGLE_BRACKET);

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a generic type node when "Generic.<ParamType>" arrived', function() {
      const typeExprStr = 'Generic.<ParamType>';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createGenericTypeNode(
        createTypeNameNode('Generic'), [
          createTypeNameNode('ParamType'),
      ], GenericTypeSyntax.ANGLE_BRACKET_WITH_DOT);

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a generic type node when "Generic.<ParamType1,ParamType2>"' +
       ' arrived', function() {
      const typeExprStr = 'Generic.<ParamType1,ParamType2>';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createGenericTypeNode(
        createTypeNameNode('Generic'), [
          createTypeNameNode('ParamType1'),
          createTypeNameNode('ParamType2'),
        ], GenericTypeSyntax.ANGLE_BRACKET_WITH_DOT);

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a generic type node when "Generic .< ParamType1 , ParamType2 >"' +
       ' arrived', function() {
      const typeExprStr = 'Generic .< ParamType1 , ParamType2 >';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createGenericTypeNode(
        createTypeNameNode('Generic'), [
          createTypeNameNode('ParamType1'),
          createTypeNameNode('ParamType2'),
        ], GenericTypeSyntax.ANGLE_BRACKET_WITH_DOT);

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a generic type node when "ParamType[]" arrived', function() {
      const typeExprStr = 'ParamType[]';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createGenericTypeNode(
        createTypeNameNode('Array'), [
          createTypeNameNode('ParamType'),
        ], GenericTypeSyntax.SQUARE_BRACKET);

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a generic type node when "ParamType[][]" arrived', function() {
      const typeExprStr = 'ParamType[][]';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createGenericTypeNode(
        createTypeNameNode('Array'), [
          createGenericTypeNode(
            createTypeNameNode('Array'), [
              createTypeNameNode('ParamType'),
          ], GenericTypeSyntax.SQUARE_BRACKET),
        ], GenericTypeSyntax.SQUARE_BRACKET);

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a generic type node when "ParamType [ ] [ ]" arrived', function() {
      const typeExprStr = 'ParamType [ ] [ ]';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createGenericTypeNode(
        createTypeNameNode('Array'), [
          createGenericTypeNode(
            createTypeNameNode('Array'), [
              createTypeNameNode('ParamType'),
          ], GenericTypeSyntax.SQUARE_BRACKET),
        ], GenericTypeSyntax.SQUARE_BRACKET);

      expect(node).to.deep.equal(expectedNode);
    });

    it('should throw a syntax error when "Promise*Error" arrived', function() {
      const typeExprStr = 'Promise*Error';

      expect(function() {
        Parser.parse(typeExprStr);
      }).to.throw(Parser.JSDocTypeSyntaxError);
    });
  });

  describe('Record types', function () {
    it('should return a record type node when "{}" arrived', function() {
      const typeExprStr = '{}';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createRecordTypeNode([]);

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a record type node when "{key:ValueType}" arrived', function() {
      const typeExprStr = '{key:ValueType}';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createRecordTypeNode([
        createRecordEntryNode('key', createTypeNameNode('ValueType')),
      ]);

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a record type node when "{keyOnly}" arrived', function() {
      const typeExprStr = '{keyOnly}';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createRecordTypeNode([
        createRecordEntryNode('keyOnly', null),
      ]);

      expect(node).to.deep.equal(expectedNode);
    });

    it('should return a record type node when \'{"keyOnly"}\' arrived', function() {
      const typeExprStr = '{"keyOnly"}';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createRecordTypeNode([
        createRecordEntryNode('keyOnly', null, 'double'),
      ]);

      expect(node).to.deep.equal(expectedNode);
    });

    it('should return a record type node when "{\'keyOnly\'}" arrived', function() {
      const typeExprStr = "{'keyOnly'}";
      const node = Parser.parse(typeExprStr);

      const expectedNode = createRecordTypeNode([
        createRecordEntryNode('keyOnly', null, 'single'),
      ]);

      expect(node).to.deep.equal(expectedNode);
    });

    it('should return a record type node when \'{"ke\\\\y\\Only\\""}\' arrived', function() {
      const typeExprStr = '{"ke\\\\y\\Only\\""}';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createRecordTypeNode([
        createRecordEntryNode('ke\\y\\Only"', null, 'double'),
      ]);

      expect(node).to.deep.equal(expectedNode);
    });

    it('should throw when \'{"key"Only"}\' arrived', function() {
      const typeExprStr = '{"key"Only"}';
      expect(function () {
        Parser.parse(typeExprStr);
      }).to.throw('Expected ",", ":", "?", "}", or [ \\t\\r\\n ] but "O" found.');
    });

    it('should throw when \'{"key\\\\"Only"}\' arrived', function() {
      const typeExprStr = '{"key\\\\"Only"}';
      expect(function () {
        Parser.parse(typeExprStr);
      }).to.throw('Expected ",", ":", "?", "}", or [ \\t\\r\\n ] but "O" found.');
    });


    it('should return a record type node when "{key1:ValueType1,key2:ValueType2}"' +
       ' arrived', function() {
      const typeExprStr = '{key1:ValueType1,key2:ValueType2}';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createRecordTypeNode([
        createRecordEntryNode('key1', createTypeNameNode('ValueType1')),
        createRecordEntryNode('key2', createTypeNameNode('ValueType2')),
      ]);

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a record type node when "{key:ValueType1,keyOnly}"' +
       ' arrived', function() {
      const typeExprStr = '{key:ValueType1,keyOnly}';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createRecordTypeNode([
        createRecordEntryNode('key', createTypeNameNode('ValueType1')),
        createRecordEntryNode('keyOnly', null),
      ]);

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a record type node when "{ key1 : ValueType1 , key2 : ValueType2 }"' +
       ' arrived', function() {
      const typeExprStr = '{ key1 : ValueType1 , key2 : ValueType2 }';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createRecordTypeNode([
        createRecordEntryNode('key1', createTypeNameNode('ValueType1')),
        createRecordEntryNode('key2', createTypeNameNode('ValueType2')),
      ]);

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a record type node when "{\'quoted-key\':ValueType}"' +
       ' arrived', function() {
      const typeExprStr = '{\'quoted-key\':ValueType}';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createRecordTypeNode([
        createRecordEntryNode('quoted-key', createTypeNameNode('ValueType'), 'single'),
      ]);

      expect(node).to.deep.equal(expectedNode);
    });
  });

  describe('Tuple types', function () {
    it('should return a tuple type node when "[]" arrived', function() {
      const typeExprStr = '[]';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createTupleTypeNode([]);

      expect(node).to.deep.equal(expectedNode);
    });

    it('should return a tuple type node when "[TupleType]" arrived', function() {
      const typeExprStr = '[TupleType]';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createTupleTypeNode([
        createTypeNameNode('TupleType'),
      ]);

      expect(node).to.deep.equal(expectedNode);
    });

    it('should return a tuple type node when "[TupleType1, TupleType2]" arrived', function() {
      const typeExprStr = '[TupleType1, TupleType2]';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createTupleTypeNode([
        createTypeNameNode('TupleType1'),
        createTypeNameNode('TupleType2'),
      ]);

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a tuple type node when "[TupleConcreteType, ...TupleVarargsType]" arrived', function() {
      const typeExprStr = '[TupleConcreteType, ...TupleVarargsType]';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createTupleTypeNode([
        createTypeNameNode('TupleConcreteType'),
        createVariadicTypeNode(
          createTypeNameNode('TupleVarargsType'),
          VariadicTypeSyntax.PREFIX_DOTS
        ),
      ]);

      expect(node).to.deep.equal(expectedNode);
    });

    it('should return a tuple type node when "[TupleConcreteType, TupleBrokenVarargsType...]" arrived', function() {
      const typeExprStr = '[TupleConcreteType, TupleBrokenVarargsType...]';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createTupleTypeNode([
        createTypeNameNode('TupleConcreteType'),
        createVariadicTypeNode(
          // This is broken because the TypeScript JSDoc parser doesn't support
          // the suffix dots variadic type syntax:
          createTypeNameNode('TupleBrokenVarargsType'),
          VariadicTypeSyntax.SUFFIX_DOTS
        ),
      ]);

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a tuple type node when "[TupleAnyVarargs, ...]" arrived', function() {
      const typeExprStr = '[TupleAnyVarargs, ...]';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createTupleTypeNode([
        createTypeNameNode('TupleAnyVarargs'),
        createVariadicTypeNode(
          null,
          VariadicTypeSyntax.ONLY_DOTS
        ),
      ]);

      expect(node).to.deep.equal(expectedNode);
    });
  });

  describe('Function/Constructor/Arrow types', function () {
    it('should return a function type node when "function()" arrived', function() {
      const typeExprStr = 'function()';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createFunctionTypeNode(
        [], null,
        { 'this': null, 'new': null }
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a function type node with a param when "function(Param)" arrived', function() {
      const typeExprStr = 'function(Param)';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createFunctionTypeNode(
        [ createTypeNameNode('Param') ], null,
        { 'this': null, 'new': null }
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a function type node with several params when "function(Param1,Param2)"' +
       ' arrived', function() {
      const typeExprStr = 'function(Param1,Param2)';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createFunctionTypeNode(
        [ createTypeNameNode('Param1'), createTypeNameNode('Param2') ], null,
        { 'this': null, 'new': null }
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a function type node with variadic params when "function(...VariadicParam)"' +
       ' arrived', function() {
      const typeExprStr = 'function(...VariadicParam)';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createFunctionTypeNode(
        [
          createVariadicTypeNode(
            createTypeNameNode('VariadicParam'),
            VariadicTypeSyntax.PREFIX_DOTS
          ),
        ],
        null, { 'this': null, 'new': null }
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a function type node with variadic params when "function(Param,...VariadicParam)"' +
       ' arrived', function() {
      const typeExprStr = 'function(Param,...VariadicParam)';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createFunctionTypeNode(
        [
          createTypeNameNode('Param'),
          createVariadicTypeNode(
            createTypeNameNode('VariadicParam'),
            VariadicTypeSyntax.PREFIX_DOTS
          ),
        ],
        null, { 'this': null, 'new': null }
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should throw an error when "function(...VariadicParam, UnexpectedLastParam)"' +
       ' arrived', function() {
      const typeExprStr = 'function(...VariadicParam, UnexpectedLastParam)';

      expect(function() {
        Parser.parse(typeExprStr);
      }).to.throw(Parser.JSDocTypeSyntaxError);
    });


    it('should return a function type node with returns when "function():Returned"' +
       ' arrived', function() {
      const typeExprStr = 'function():Returned';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createFunctionTypeNode(
        [], createTypeNameNode('Returned'),
        { 'this': null, 'new': null }
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a function type node with a context type when "function(this:ThisObject)"' +
       ' arrived', function() {
      const typeExprStr = 'function(this:ThisObject)';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createFunctionTypeNode(
        [], null,
        { 'this': createTypeNameNode('ThisObject'), 'new': null }
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a function type node with a context type when ' +
       '"function(this:ThisObject, param1)" arrived', function() {
      const typeExprStr = 'function(this:ThisObject, param1)';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createFunctionTypeNode(
        [createTypeNameNode('param1')],
        null,
        { 'this': createTypeNameNode('ThisObject'), 'new': null }
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a function type node as a constructor when "function(new:NewObject)"' +
       ' arrived', function() {
      const typeExprStr = 'function(new:NewObject)';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createFunctionTypeNode(
        [], null,
        { 'this': null, 'new': createTypeNameNode('NewObject') }
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a function type node as a constructor when ' +
       '"function(new:NewObject, param1)" arrived', function() {
      const typeExprStr = 'function(new:NewObject, param1)';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createFunctionTypeNode(
        [createTypeNameNode('param1')],
        null,
        { 'this': null, 'new': createTypeNameNode('NewObject') }
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should throw an error when "function(new:NewObject, this:ThisObject)" ' +
       'arrived', function() {
      const typeExprStr = 'function(new:NewObject, this:ThisObject)';

      expect(function() {
        Parser.parse(typeExprStr);
      }).to.throw(Parser.JSDocTypeSyntaxError);
    });


    it('should throw an error when "function(this:ThisObject, new:NewObject)" ' +
       'arrived', function() {
      const typeExprStr = 'function(this:ThisObject, new:NewObject)';

      expect(function() {
        Parser.parse(typeExprStr);
      }).to.throw(Parser.JSDocTypeSyntaxError);
    });


    it('should return a function type node when "function( Param1 , Param2 ) : Returned"' +
       ' arrived', function() {
      const typeExprStr = 'function( Param1 , Param2 ) : Returned';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createFunctionTypeNode(
        [ createTypeNameNode('Param1'), createTypeNameNode('Param2') ],
        createTypeNameNode('Returned'),
        { 'this': null, 'new': null }
      );

      expect(node).to.deep.equal(expectedNode);
    });

    it('should return an arrow function type node when "() => string" arrived', function() {
      const typeExprStr = '() => string';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createArrowFunctionTypeNode(
        [], createTypeNameNode('string'),
        { 'new': null }
      );

      expect(node).to.deep.equal(expectedNode);
    });
  });

  describe('BroadNamepathExpr types', function () {
    describe('Single component', function () {
      it('should return a type name node when "TypeName" arrived', function() {
        const typeExprStr = 'TypeName';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createTypeNameNode(typeExprStr);
        expect(node).to.deep.equal(expectedNode);
      });


      it('should return a type name node when "my-type" arrived', function() {
        const typeExprStr = 'my-type';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createTypeNameNode(typeExprStr);
        expect(node).to.deep.equal(expectedNode);
      });


      it('should return a type name node when "$" arrived', function() {
        const typeExprStr = '$';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createTypeNameNode(typeExprStr);
        expect(node).to.deep.equal(expectedNode);
      });


      it('should return a type name node when "_" arrived', function() {
        const typeExprStr = '_';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createTypeNameNode(typeExprStr);
        expect(node).to.deep.equal(expectedNode);
      });
    });
    describe('Multipart', function () {
      it('should return a member type node when "owner.Member" arrived', function() {
        const typeExprStr = 'owner.Member';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createMemberTypeNode(
          createTypeNameNode('owner'),
          'Member'
        );

        expect(node).to.deep.equal(expectedNode);
      });


      it('should return a member type node when "owner . Member" arrived', function() {
        const typeExprStr = 'owner . Member';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createMemberTypeNode(
          createTypeNameNode('owner'),
          'Member');

        expect(node).to.deep.equal(expectedNode);
      });


      it('should return a member type node when \'owner."Mem.ber"\' arrived', function() {
        const typeExprStr = 'owner."Mem.ber"';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createMemberTypeNode(
          createTypeNameNode('owner'),
          'Mem.ber',
          'double'
        );

        expect(node).to.deep.equal(expectedNode);
      });

      it('should return a member type node when \'owner."Mem\\ber"\' arrived', function() {
        const typeExprStr = 'owner."Mem\\ber"';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createMemberTypeNode(
          createTypeNameNode('owner'),
          'Mem\\ber',
          'double'
        );

        expect(node).to.deep.equal(expectedNode);
      });

      it('should return a member type node when "owner.\'Mem\\ber\'" arrived', function() {
        const typeExprStr = 'owner.\'Mem\\ber\'';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createMemberTypeNode(
          createTypeNameNode('owner'),
          'Mem\\ber',
          'single'
        );

        expect(node).to.deep.equal(expectedNode);
      });

      it('should return a member type node when \'owner."Mem.ber\\""\' arrived', function() {
        const typeExprStr = 'owner."Mem.ber\\""';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createMemberTypeNode(
          createTypeNameNode('owner'),
          'Mem.ber"',
          'double'
        );

        expect(node).to.deep.equal(expectedNode);
      });

      it('should return a member type node when \'owner."Me\\m\\\\ber"\' arrived', function() {
        const typeExprStr = 'owner."Me\\m\\"ber\\\\"';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createMemberTypeNode(
          createTypeNameNode('owner'),
          'Me\\m"ber\\',
          'double');

        expect(node).to.deep.equal(expectedNode);
      });

      it('should return a member type node when \'owner."Mem\\\\ber\\""\' arrived', function() {
        const typeExprStr = 'owner."Mem\\\\ber\\""';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createMemberTypeNode(
          createTypeNameNode('owner'),
          'Mem\\ber"',
          'double');

        expect(node).to.deep.equal(expectedNode);
      });

      it('should throw when \'owner."Mem.ber\\"\' arrived', function() {
        const typeExprStr = 'owner."Mem.ber\\"';
        expect(function () {
          Parser.parse(typeExprStr);
        }).to.throw('Expected "\\""');
      });


      it('should return a member type node when "superOwner.owner.Member" arrived', function() {
        const typeExprStr = 'superOwner.owner.Member';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createMemberTypeNode(
            createMemberTypeNode(
              createTypeNameNode('superOwner'), 'owner'),
            'Member');

        expect(node).to.deep.equal(expectedNode);
      });


      it('should return a member type node when "superOwner.owner.Member=" arrived', function() {
        const typeExprStr = 'superOwner.owner.Member=';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createOptionalTypeNode(
          createMemberTypeNode(
            createMemberTypeNode(
              createTypeNameNode('superOwner'),
            'owner'),
          'Member'),
          OptionalTypeSyntax.SUFFIX_EQUALS_SIGN
        );

        expect(node).to.deep.equal(expectedNode);
      });


      it('should return an inner member type node when "owner~innerMember" arrived', function() {
        const typeExprStr = 'owner~innerMember';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createInnerMemberTypeNode(
          createTypeNameNode('owner'),
          'innerMember');

        expect(node).to.deep.equal(expectedNode);
      });


      it('should return an inner member type node when "owner ~ innerMember" arrived', function() {
        const typeExprStr = 'owner ~ innerMember';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createInnerMemberTypeNode(
          createTypeNameNode('owner'),
          'innerMember');

        expect(node).to.deep.equal(expectedNode);
      });


      it('should return an inner member type node when "superOwner~owner~innerMember" ' +
         'arrived', function() {
        const typeExprStr = 'superOwner~owner~innerMember';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createInnerMemberTypeNode(
            createInnerMemberTypeNode(
              createTypeNameNode('superOwner'), 'owner'),
            'innerMember');

        expect(node).to.deep.equal(expectedNode);
      });


      it('should return an inner member type node when "superOwner~owner~innerMember=" ' +
         'arrived', function() {
        const typeExprStr = 'superOwner~owner~innerMember=';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createOptionalTypeNode(
          createInnerMemberTypeNode(
            createInnerMemberTypeNode(
              createTypeNameNode('superOwner'),
            'owner'),
          'innerMember'),
          OptionalTypeSyntax.SUFFIX_EQUALS_SIGN
        );

        expect(node).to.deep.equal(expectedNode);
      });


      it('should return an instance member type node when "owner#instanceMember" arrived', function() {
        const typeExprStr = 'owner#instanceMember';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createInstanceMemberTypeNode(
          createTypeNameNode('owner'),
          'instanceMember');

        expect(node).to.deep.equal(expectedNode);
      });


      it('should return an instance member type node when "owner # instanceMember" ' +
         'arrived', function() {
        const typeExprStr = 'owner # instanceMember';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createInstanceMemberTypeNode(
          createTypeNameNode('owner'),
          'instanceMember');

        expect(node).to.deep.equal(expectedNode);
      });


      it('should return an instance member type node when "superOwner#owner#instanceMember" ' +
         'arrived', function() {
        const typeExprStr = 'superOwner#owner#instanceMember';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createInstanceMemberTypeNode(
            createInstanceMemberTypeNode(
              createTypeNameNode('superOwner'), 'owner'),
            'instanceMember');

        expect(node).to.deep.equal(expectedNode);
      });


      it('should return an instance member type node when "superOwner#owner#instanceMember=" ' +
         'arrived', function() {
        const typeExprStr = 'superOwner#owner#instanceMember=';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createOptionalTypeNode(
          createInstanceMemberTypeNode(
            createInstanceMemberTypeNode(
              createTypeNameNode('superOwner'),
            'owner'),
          'instanceMember'),
          OptionalTypeSyntax.SUFFIX_EQUALS_SIGN
        );

        expect(node).to.deep.equal(expectedNode);
      });


      it('should return a member type node when "owner.event:Member" arrived', function() {
        const typeExprStr = 'owner.event:Member';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createMemberTypeNode(
          createTypeNameNode('owner'),
          'Member',
          'none',
          true);

        expect(node).to.deep.equal(expectedNode);
      });
    });
    describe('external', function () {
      it('should return an external name node when "external:string" arrived', function() {
        const typeExprStr = 'external:string';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createExternalNameNode('string');
        expect(node).to.deep.equal(expectedNode);
      });


      it('should return an external name node when "external : String#rot13" arrived', function() {
        const typeExprStr = 'external : String#rot13';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createInstanceMemberTypeNode(
          createExternalNameNode('String'),
          'rot13'
        );
        expect(node).to.deep.equal(expectedNode);
      });

      it('should return an external name node when `external:"jQuery.fn"` arrived', function() {
        const typeExprStr = 'external:"jQuery.fn"';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createExternalNameNode('jQuery.fn', 'double');
        expect(node).to.deep.equal(expectedNode);
      });

      it('should return an external name node when `external:"jQuery.fn".someMethod#event:abc` arrived', function() {
        const typeExprStr = 'external:"jQuery.fn".someMethod#event:abc';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createInstanceMemberTypeNode(
          createMemberTypeNode(
            createExternalNameNode('jQuery.fn', 'double'),
            'someMethod'
          ),
          'abc',
          'none',
          true
        );
        expect(node).to.deep.equal(expectedNode);
      });
    });

    describe('module', function () {
      it('should return a module name node when "module:path/to/file" arrived', function() {
        const typeExprStr = 'module:path/to/file';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createModuleNameNode(createFilePathNode('path/to/file'));
        expect(node).to.deep.equal(expectedNode);
      });


      it('should return a module name node when "module : path/to/file" arrived', function() {
        const typeExprStr = 'module : path/to/file';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createModuleNameNode(createFilePathNode('path/to/file'));
        expect(node).to.deep.equal(expectedNode);
      });


      it('should return a member node when "(module:path/to/file).member" arrived', function() {
        const typeExprStr = '(module:path/to/file).member';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createMemberTypeNode(
          createParenthesizedNode(
            createModuleNameNode(createFilePathNode('path/to/file'))
          ),
          'member'
        );
        expect(node).to.deep.equal(expectedNode);
      });


      it('should return a member node when "module:path/to/file.member" arrived', function() {
        const typeExprStr = 'module:path/to/file.member';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createModuleNameNode(
          createMemberTypeNode(createFilePathNode('path/to/file'), 'member')
        );
        expect(node).to.deep.equal(expectedNode);
      });

      it('should return a member node when "module:path/to/file.event:member" arrived', function() {
        const typeExprStr = 'module:path/to/file.event:member';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createModuleNameNode(
          createMemberTypeNode(createFilePathNode('path/to/file'), 'member', 'none', true)
        );
        expect(node).to.deep.equal(expectedNode);
      });

      it('should return a member node when \'module:"path/to/file".event:member\' arrived', function() {
        const typeExprStr = 'module:"path/to/file".event:member';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createModuleNameNode(
          createMemberTypeNode(createFilePathNode('path/to/file', 'double'), 'member', 'none', true)
        );
        expect(node).to.deep.equal(expectedNode);
      });

      it('should return a member node when \'module:"path\\\\to\\file\\"".event:member\' arrived', function() {
        const typeExprStr = 'module:"path\\\\to\\file\\"".event:member';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createModuleNameNode(
          createMemberTypeNode(createFilePathNode('path\\to\\file"', 'double'), 'member', 'none', true)
        );
        expect(node).to.deep.equal(expectedNode);
      });

      it('should throw when \'module:"path/t"o/file".event:member\' arrived', function() {
        const typeExprStr = 'module:"path/t"o/file".event:member';
        expect(function () {
          Parser.parse(typeExprStr);
        }).to.throw('Expected "!", "#", ".", "...", ".<", "/", "<", "=", "?", "[", "|", "~", [ \\t\\r\\n ], or end of input but "o" found.');
      });

      it('should throw when \'module:"path/t\\\\"o/file".event:member', function() {
        const typeExprStr = 'module:"path/t\\\\"o/file".event:member';
        expect(function () {
          Parser.parse(typeExprStr);
        }).to.throw('Expected "!", "#", ".", "...", ".<", "/", "<", "=", "?", "[", "|", "~", [ \\t\\r\\n ], or end of input but "o" found.');
      });

      it('should return a member node when "module:\'path/to/file\'.event:member" arrived', function() {
        const typeExprStr = "module:'path/to/file'.event:member";
        const node = Parser.parse(typeExprStr);

        const expectedNode = createModuleNameNode(
          createMemberTypeNode(createFilePathNode('path/to/file', 'single'), 'member', 'none', true)
        );
        expect(node).to.deep.equal(expectedNode);
      });
    });
  });

  describe('Types with modifiers', function () {
    it('should return an optional unknown type node when "?=" arrived', function() {
      const typeExprStr = '?=';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createOptionalTypeNode(
        createUnknownTypeNode(),
        OptionalTypeSyntax.SUFFIX_EQUALS_SIGN
      );
      expect(node).to.deep.equal(expectedNode);
    });

    it('should return an optional type node when "string=" arrived', function() {
      const typeExprStr = 'string=';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createOptionalTypeNode(
        createTypeNameNode('string'),
        OptionalTypeSyntax.SUFFIX_EQUALS_SIGN
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return an optional type node when "=string" arrived (deprecated)', function() {
      const typeExprStr = '=string';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createOptionalTypeNode(
        createTypeNameNode('string'),
        OptionalTypeSyntax.PREFIX_EQUALS_SIGN
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a nullable type node when "?string" arrived', function() {
      const typeExprStr = '?string';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createNullableTypeNode(
        createTypeNameNode('string'),
        NullableTypeSyntax.PREFIX_QUESTION_MARK
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a nullable type node when "string?" arrived (deprecated)', function() {
      const typeExprStr = 'string?';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createNullableTypeNode(
        createTypeNameNode('string'),
        NullableTypeSyntax.SUFFIX_QUESTION_MARK
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return an optional type node when "string =" arrived', function() {
      const typeExprStr = 'string =';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createOptionalTypeNode(
        createTypeNameNode('string'),
        OptionalTypeSyntax.SUFFIX_EQUALS_SIGN
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return an optional type node when "= string" arrived (deprecated)', function() {
      const typeExprStr = '= string';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createOptionalTypeNode(
        createTypeNameNode('string'),
        OptionalTypeSyntax.PREFIX_EQUALS_SIGN
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a nullable type node when "? string" arrived', function() {
      const typeExprStr = '? string';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createNullableTypeNode(
        createTypeNameNode('string'),
        NullableTypeSyntax.PREFIX_QUESTION_MARK
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a nullable type node when "string ?" arrived (deprecated)', function() {
      const typeExprStr = 'string ?';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createNullableTypeNode(
        createTypeNameNode('string'),
        NullableTypeSyntax.SUFFIX_QUESTION_MARK
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return an optional type node when "?string=" arrived', function() {
      const typeExprStr = '?string=';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createOptionalTypeNode(
        createNullableTypeNode(
          createTypeNameNode('string'),
          NullableTypeSyntax.PREFIX_QUESTION_MARK
        ),
        OptionalTypeSyntax.SUFFIX_EQUALS_SIGN
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return an optional type node when "string?=" arrived', function() {
      const typeExprStr = 'string?=';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createOptionalTypeNode(
        createNullableTypeNode(
          createTypeNameNode('string'),
          NullableTypeSyntax.SUFFIX_QUESTION_MARK
        ),
        OptionalTypeSyntax.SUFFIX_EQUALS_SIGN
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should throw an error when "?!string" arrived', function() {
      const typeExprStr = '?!string';

      expect(function() {
        Parser.parse(typeExprStr);
      }).to.throw(Parser.JSDocTypeSyntaxError);
    });


    it('should throw an error when "!?string" arrived', function() {
      const typeExprStr = '!?string';

      expect(function() {
        Parser.parse(typeExprStr);
      }).to.throw(Parser.JSDocTypeSyntaxError);
    });

    it('should return a variadic type node when "...PrefixVariadic" arrived', function() {
      const typeExprStr = '...PrefixVariadic';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createVariadicTypeNode(
        createTypeNameNode('PrefixVariadic'),
        VariadicTypeSyntax.PREFIX_DOTS
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a variadic type node when "SuffixVariadic..." arrived', function() {
      const typeExprStr = 'SuffixVariadic...';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createVariadicTypeNode(
        createTypeNameNode('SuffixVariadic'),
        VariadicTypeSyntax.SUFFIX_DOTS
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a variadic type node when "..." arrived', function() {
      const typeExprStr = '...';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createVariadicTypeNode(
        null,
        VariadicTypeSyntax.ONLY_DOTS
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a variadic type node when "...!Object" arrived', function() {
      const typeExprStr = '...!Object';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createVariadicTypeNode(
        createNotNullableTypeNode(
          createTypeNameNode('Object'),
          NotNullableTypeSyntax.PREFIX_BANG
        ),
        VariadicTypeSyntax.PREFIX_DOTS
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a variadic type node when "...?" arrived', function() {
      const typeExprStr = '...?';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createVariadicTypeNode(
        createUnknownTypeNode(),
        VariadicTypeSyntax.PREFIX_DOTS
      );

      expect(node).to.deep.equal(expectedNode);
    });

    it('should return a not nullable type node when "!Object" arrived', function() {
      const typeExprStr = '!Object';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createNotNullableTypeNode(
        createTypeNameNode('Object'),
        NotNullableTypeSyntax.PREFIX_BANG
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a not nullable type node when "Object!" arrived', function() {
      const typeExprStr = 'Object!';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createNotNullableTypeNode(
        createTypeNameNode('Object'),
        NotNullableTypeSyntax.SUFFIX_BANG
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a not nullable type node when "! Object" arrived', function() {
      const typeExprStr = '! Object';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createNotNullableTypeNode(
        createTypeNameNode('Object'),
        NotNullableTypeSyntax.PREFIX_BANG
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a not nullable type node when "Object !" arrived', function() {
      const typeExprStr = 'Object !';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createNotNullableTypeNode(
        createTypeNameNode('Object'),
        NotNullableTypeSyntax.SUFFIX_BANG
      );

      expect(node).to.deep.equal(expectedNode);
    });
  });

  describe('Type combinations', function () {

    it('should return a union type when "LeftType|RightType" arrived', function() {
      const typeExprStr = 'LeftType|RightType';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createUnionTypeNode(
        createTypeNameNode('LeftType'),
        createTypeNameNode('RightType'),
        UnionTypeSyntax.PIPE
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a union type when "LeftType|MiddleType|RightType" arrived', function() {
      const typeExprStr = 'LeftType|MiddleType|RightType';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createUnionTypeNode(
        createTypeNameNode('LeftType'),
        createUnionTypeNode(
          createTypeNameNode('MiddleType'),
          createTypeNameNode('RightType'),
          UnionTypeSyntax.PIPE
        ), UnionTypeSyntax.PIPE);

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a union type when "(LeftType|RightType)" arrived', function() {
      const typeExprStr = '(LeftType|RightType)';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createParenthesizedNode(
        createUnionTypeNode(
          createTypeNameNode('LeftType'),
          createTypeNameNode('RightType')
        )
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a union type when "( LeftType | RightType )" arrived', function() {
      const typeExprStr = '( LeftType | RightType )';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createParenthesizedNode(
        createUnionTypeNode(
          createTypeNameNode('LeftType'),
          createTypeNameNode('RightType')
        )
      );

      expect(node).to.deep.equal(expectedNode);
    });


    it('should return a union type when "LeftType/RightType" arrived', function() {
      const typeExprStr = 'LeftType/RightType';
      const node = Parser.parse(typeExprStr);

      const expectedNode = createUnionTypeNode(
        createTypeNameNode('LeftType'),
        createTypeNameNode('RightType'),
        UnionTypeSyntax.SLASH
      );

      expect(node).to.deep.equal(expectedNode);
    });

    it('should throw a syntax error when "(unclosedParenthesis, " arrived', function() {
      const typeExprStr = '(unclosedParenthesis, ';

      expect(function() {
        Parser.parse(typeExprStr);
      }).to.throw(Parser.JSDocTypeSyntaxError);
    });
  });


  it('should return a type query type node when "typeof foo" arrived', function() {
    const typeExprStr = 'typeof foo';
    const node = Parser.parse(typeExprStr);

    const expectedNode = createTypeQueryNode(
      createTypeNameNode('foo')
    );
    expect(node).to.deep.equal(expectedNode);
  });


  it('should return a key query type node when "keyof foo" arrived', function() {
    const typeExprStr = 'keyof foo';
    const node = Parser.parse(typeExprStr);

    const expectedNode = createKeyQueryNode(
      createTypeNameNode('foo')
    );
    expect(node).to.deep.equal(expectedNode);
  });

  it('should return a key query type node when "keyof typeof foo" arrived', function() {
    const typeExprStr = 'keyof typeof foo';
    const node = Parser.parse(typeExprStr);

    const expectedNode = createKeyQueryNode(
      createTypeQueryNode(
        createTypeNameNode('foo')
      )
    );
    expect(node).to.deep.equal(expectedNode);
  });


  describe('operator precedence', function() {
    context('when "Foo|function():Returned?" arrived', function() {
      it('should parse as "Foo|((function():Returned)?)"', function() {
        const typeExprStr = 'Foo|function():Returned?';
        const node = Parser.parse(typeExprStr);

        const expectedNode = createUnionTypeNode(
          createTypeNameNode('Foo'),
          createNullableTypeNode(
            createFunctionTypeNode(
              [],
              createTypeNameNode('Returned'),
            { 'this': null, 'new': null }
            ),
            NullableTypeSyntax.SUFFIX_QUESTION_MARK
          ),
          UnionTypeSyntax.PIPE
        );

        expect(node).to.deep.equal(expectedNode);
      });
    });
  });
});


/**
 * @param {string} typeName
 */
function createTypeNameNode(typeName) {
  return {
    type: NodeType.NAME,
    name: typeName,
  };
}

function createAnyTypeNode() {
  return {
    type: NodeType.ANY,
  };
}

function createUnknownTypeNode() {
  return {
    type: NodeType.UNKNOWN,
  };
}

function createModuleNameNode(value) {
  return {
    type: NodeType.MODULE,
    value: value,
  };
}

function createExternalNameNode(name, quoteStyle = 'none') {
  return {
    type: NodeType.EXTERNAL,
    name,
    quoteStyle,
  };
}

function createOptionalTypeNode(optionalTypeExpr, syntax) {
  return {
    type: NodeType.OPTIONAL,
    value: optionalTypeExpr,
    meta: { syntax: syntax },
  };
}

function createNullableTypeNode(nullableTypeExpr, syntax) {
  return {
    type: NodeType.NULLABLE,
    value: nullableTypeExpr,
    meta: { syntax: syntax },
  };
}

function createNotNullableTypeNode(nullableTypeExpr, syntax) {
  return {
    type: NodeType.NOT_NULLABLE,
    value: nullableTypeExpr,
    meta: { syntax: syntax },
  };
}

function createMemberTypeNode(ownerTypeExpr, memberTypeName, quoteStyle = 'none', hasEventPrefix = false) {
  return {
    type: NodeType.MEMBER,
    owner: ownerTypeExpr,
    name: memberTypeName,
    quoteStyle,
    hasEventPrefix,
  };
}

function createInnerMemberTypeNode(ownerTypeExpr, memberTypeName, quoteStyle = 'none', hasEventPrefix = false) {
  return {
    type: NodeType.INNER_MEMBER,
    owner: ownerTypeExpr,
    name: memberTypeName,
    quoteStyle,
    hasEventPrefix,
  };
}

function createInstanceMemberTypeNode(ownerTypeExpr, memberTypeName, quoteStyle = 'none', hasEventPrefix = false) {
  return {
    type: NodeType.INSTANCE_MEMBER,
    owner: ownerTypeExpr,
    name: memberTypeName,
    quoteStyle,
    hasEventPrefix,
  };
}

function createUnionTypeNode(leftTypeExpr, rightTypeExpr, syntax) {
  return {
    type: NodeType.UNION,
    left: leftTypeExpr,
    right: rightTypeExpr,
    meta: { syntax: syntax || UnionTypeSyntax.PIPE },
  };
}

function createVariadicTypeNode(variadicTypeExpr, syntax) {
  return {
    type: NodeType.VARIADIC,
    value: variadicTypeExpr,
    meta: { syntax: syntax },
  };
}

function createRecordTypeNode(recordEntries) {
  return {
    type: NodeType.RECORD,
    entries: recordEntries,
  };
}

function createRecordEntryNode(key, valueTypeExpr, quoteStyle = 'none') {
  return {
    type: NodeType.RECORD_ENTRY,
    key: key,
    value: valueTypeExpr,
    quoteStyle,
  };
}

/**
 * @template {Node} T
 * @param {T[]} tupleEntries
 */
function createTupleTypeNode(tupleEntries) {
  return {
    type: NodeType.TUPLE,
    entries: tupleEntries,
  };
}

function createGenericTypeNode(genericTypeName, paramExprs, syntaxType) {
  return {
    type: NodeType.GENERIC,
    subject: genericTypeName,
    objects: paramExprs,
    meta: { syntax: syntaxType || GenericTypeSyntax.ANGLE_BRACKET },
  };
}

function createArrowFunctionTypeNode(paramNodes, returnedNode, modifierMap) {
  return {
    type: NodeType.ARROW,
    params: paramNodes,
    returns: returnedNode,
    new: modifierMap.new,
  };
}

function createFunctionTypeNode(paramNodes, returnedNode, modifierMap) {
  return {
    type: NodeType.FUNCTION,
    params: paramNodes,
    returns: returnedNode,
    this: modifierMap.this,
    new: modifierMap.new,
  };
}

function createNumberValueNode(numberLiteral) {
  return {
    type: NodeType.NUMBER_VALUE,
    number: numberLiteral,
  };
}

function createStringValueNode(stringLiteral, quoteStyle = 'double') {
  return {
    type: NodeType.STRING_VALUE,
    quoteStyle,
    string: stringLiteral,
  };
}

function createFilePathNode(filePath, quoteStyle = 'none') {
  return {
    type: NodeType.FILE_PATH,
    path: filePath,
    quoteStyle,
  };
}

function createParenthesizedNode(value) {
  return {
    type: NodeType.PARENTHESIS,
    value: value,
  };
}

/**
 * @template {Node} T
 * @param {T} name
 */
function createTypeQueryNode(name) {
  return {
    type: NodeType.TYPE_QUERY,
    name: name,
  }
}

/**
 * @template {Node} T
 * @param {T} value
 */
function createKeyQueryNode(value) {
  return {
    type: NodeType.KEY_QUERY,
    value: value,
  };
}
