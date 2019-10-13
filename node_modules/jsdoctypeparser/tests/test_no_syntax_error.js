'use strict';

const fs = require('fs');
const path = require('path');
const util = require('util');
const Parser = require('../lib/parsing.js');

const Fixtures = {
  CATHARSIS: readFixtureSync('catharsis-types'),
  CLOSURE_LIBRARY: readFixtureSync('closure-library-types'),
  JSDOC3: readFixtureSync('jsdoc-types'),
  JSDUCK: readFixtureSync('jsduck-types'),
  TYPESCRIPT: readFixtureSync('typescript-types'),
};

describe('Parser', function() {
  it('should not throw any errors when parsing tests/fixtures/*', function() {
    Object.keys(Fixtures).forEach(function(fixtureName) {
      Fixtures[fixtureName].forEach(function({skip, typeExprStr, position}) {
        if (skip) return;

        try {
          Parser.parse(typeExprStr);
        }
        catch (e) {
          const debugMessage = util.format('parsing %s at %s:%d\n\n%s',
                                         typeExprStr,
                                         position.filePath,
                                         position.lineno,
                                         e.stack);

          throw new Error(debugMessage);
        }
      });
    });
  });
});


function readFixtureSync(fileName) {
  const filePath = path.resolve(__dirname, 'fixtures', fileName);

  return fs.readFileSync(filePath, 'utf8')
    .trim()
    .split(/\n/)
    .map(function(line, lineIdx) {
      return {
        // When the line starts with "//", we should skip it.
        skip: /^\/\//.test(line),

        typeExprStr: line.trim().replace(/^\{(.*)\}$/, '$1').replace(/\\n/g, '\n'),
        position: {
          filePath,
          lineno: lineIdx + 1,
        },
      };
    });
}
