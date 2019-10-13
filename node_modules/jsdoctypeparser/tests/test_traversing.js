'use strict';

const {expect} = require('chai');
const entries = require('object.entries-ponyfill');

const NodeType = require('../lib/NodeType.js');
const {traverse} = require('../lib/traversing.js');

/** @typedef {{type: import('../lib/NodeType').Type}} Node */

describe('traversing', function() {
  const testCaseGroups = {
    'Primitive types': {
      'should visit a string value node': {
        given: { type: NodeType.STRING_VALUE, value: 'stringValue' },
        then: [
          ['enter', NodeType.STRING_VALUE, null, null],
          ['leave', NodeType.STRING_VALUE, null, null],
        ],
      },

      'should visit a number value node': {
        given: { type: NodeType.NUMBER_VALUE, value: 'numberValue' },
        then: [
          ['enter', NodeType.NUMBER_VALUE, null, null],
          ['leave', NodeType.NUMBER_VALUE, null, null],
        ],
      },
    },
    'Wildcard types': {
      'should visit an any node': {
        given: {
          type: NodeType.ANY,
        },
        then: [
          ['enter', NodeType.ANY, null, null],
          ['leave', NodeType.ANY, null, null],
        ],
      },

      'should visit an unknown node': {
        given: {
          type: NodeType.UNKNOWN,
        },
        then: [
          ['enter', NodeType.UNKNOWN, null, null],
          ['leave', NodeType.UNKNOWN, null, null],
        ],
      },
    },
    'Generic types': {
      'should visit a generic node that is empty': {
        given: {
          type: NodeType.GENERIC,
          subject: createNameNode('subject'),
          objects: [],
        },
        then: [
          ['enter', NodeType.GENERIC, null, null],
          ['enter', NodeType.NAME, 'subject', NodeType.GENERIC],
          ['leave', NodeType.NAME, 'subject', NodeType.GENERIC],
          ['leave', NodeType.GENERIC, null, null],
        ],
      },

      'should visit a generic node that has multiple objects': {
        given: {
          type: NodeType.GENERIC,
          subject: createNameNode('subject'),
          objects: [
            createNameNode('object1'),
            createNameNode('object2'),
          ],
        },
        then: [
          ['enter', NodeType.GENERIC, null, null],
          ['enter', NodeType.NAME, 'subject', NodeType.GENERIC],
          ['leave', NodeType.NAME, 'subject', NodeType.GENERIC],
          ['enter', NodeType.NAME, 'objects', NodeType.GENERIC],
          ['leave', NodeType.NAME, 'objects', NodeType.GENERIC],
          ['enter', NodeType.NAME, 'objects', NodeType.GENERIC],
          ['leave', NodeType.NAME, 'objects', NodeType.GENERIC],
          ['leave', NodeType.GENERIC, null, null],
        ],
      },
    },
    'Record types': {
      'should visit a record node that is empty': {
        given: {
          type: NodeType.RECORD,
          entries: [],
        },
        then: [
          ['enter', NodeType.RECORD, null, null],
          ['leave', NodeType.RECORD, null, null],
        ],
      },

      'should visit a record node that has multiple entries': {
        given: {
          type: NodeType.RECORD,
          entries: [
            createRecordEntry('key1', createNameNode('key1')),
            createRecordEntry('key2', createNameNode('key2')),
          ],
        },
        then: [
          ['enter', NodeType.RECORD, null, null],
          ['enter', NodeType.RECORD_ENTRY, 'entries', NodeType.RECORD],
          ['enter', NodeType.NAME, 'value', NodeType.RECORD_ENTRY],
          ['leave', NodeType.NAME, 'value', NodeType.RECORD_ENTRY],
          ['leave', NodeType.RECORD_ENTRY, 'entries', NodeType.RECORD],
          ['enter', NodeType.RECORD_ENTRY, 'entries', NodeType.RECORD],
          ['enter', NodeType.NAME, 'value', NodeType.RECORD_ENTRY],
          ['leave', NodeType.NAME, 'value', NodeType.RECORD_ENTRY],
          ['leave', NodeType.RECORD_ENTRY, 'entries', NodeType.RECORD],
          ['leave', NodeType.RECORD, null, null],
        ],
      },
    },
    'Tuple types': {
      'should visit a tuple node that is empty': {
        given: {
          type: NodeType.TUPLE,
          entries: [],
        },
        then: [
          ['enter', NodeType.TUPLE, null, null],
          ['leave', NodeType.TUPLE, null, null],
        ],
      },

      'should visit a tuple node that has multiple entries': {
        given: {
          type: NodeType.TUPLE,
          entries: [
            createNameNode('object1'),
            createNameNode('object2'),
          ],
        },
        then: [
          ['enter', NodeType.TUPLE, null, null],
          ['enter', NodeType.NAME, 'entries', NodeType.TUPLE],
          ['leave', NodeType.NAME, 'entries', NodeType.TUPLE],
          ['enter', NodeType.NAME, 'entries', NodeType.TUPLE],
          ['leave', NodeType.NAME, 'entries', NodeType.TUPLE],
          ['leave', NodeType.TUPLE, null, null],
        ],
      },
    },
    'Function/Constructor/Arrow types': {
      'should visit a function node that has no params and no returns': {
        given: {
          type: NodeType.FUNCTION,
          params: [],
          returns: null,
          this: null,
          new: null,
        },
        then: [
          ['enter', NodeType.FUNCTION, null, null],
          ['leave', NodeType.FUNCTION, null, null],
        ],
      },

      'should visit a function node that has few params and a returns and "this" and "new"': {
        given: {
          type: NodeType.FUNCTION,
          params: [
            createNameNode('param1'),
            createNameNode('param2'),
          ],
          returns: createNameNode('return'),
          this: createNameNode('this'),
          new: createNameNode('new'),
        },
        then: [
          ['enter', NodeType.FUNCTION, null, null],
          ['enter', NodeType.NAME, 'params', NodeType.FUNCTION],
          ['leave', NodeType.NAME, 'params', NodeType.FUNCTION],
          ['enter', NodeType.NAME, 'params', NodeType.FUNCTION],
          ['leave', NodeType.NAME, 'params', NodeType.FUNCTION],
          ['enter', NodeType.NAME, 'returns', NodeType.FUNCTION],
          ['leave', NodeType.NAME, 'returns', NodeType.FUNCTION],
          ['enter', NodeType.NAME, 'this', NodeType.FUNCTION],
          ['leave', NodeType.NAME, 'this', NodeType.FUNCTION],
          ['enter', NodeType.NAME, 'new', NodeType.FUNCTION],
          ['leave', NodeType.NAME, 'new', NodeType.FUNCTION],
          ['leave', NodeType.FUNCTION, null, null],
        ],
      },

      'should visit an arrow function that has two params and a returns': {
        given: {
          type: NodeType.ARROW,
          params: [
            { type: NodeType.NAMED_PARAMETER, name: 'param1', typeName: createNameNode('type1') },
            { type: NodeType.NAMED_PARAMETER, name: 'param2', typeName: createNameNode('type2') },
          ],
          returns: createNameNode('return'),
        },
        then: [
          ['enter', NodeType.ARROW, null, null],
          ['enter', NodeType.NAMED_PARAMETER, 'params', NodeType.ARROW],
          ['enter', NodeType.NAME, 'typeName', NodeType.NAMED_PARAMETER],
          ['leave', NodeType.NAME, 'typeName', NodeType.NAMED_PARAMETER],
          ['leave', NodeType.NAMED_PARAMETER, 'params', NodeType.ARROW],
          ['enter', NodeType.NAMED_PARAMETER, 'params', NodeType.ARROW],
          ['enter', NodeType.NAME, 'typeName', NodeType.NAMED_PARAMETER],
          ['leave', NodeType.NAME, 'typeName', NodeType.NAMED_PARAMETER],
          ['leave', NodeType.NAMED_PARAMETER, 'params', NodeType.ARROW],
          ['enter', NodeType.NAME, 'returns', NodeType.ARROW],
          ['leave', NodeType.NAME, 'returns', NodeType.ARROW],
          ['leave', NodeType.ARROW, null, null],
        ],
      },

      'should visit an arrow function that has one variadic param and a returns': {
        given: {
          type: NodeType.ARROW,
          params: [
            {
              type: NodeType.VARIADIC,
              value: {
                type: NodeType.NAMED_PARAMETER,
                name: 'param1',
                typeName: createNameNode('type1'),
              },
            },
          ],
          returns: createNameNode('return'),
        },
        then: [
          ['enter', NodeType.ARROW, null, null],
          ['enter', NodeType.VARIADIC, 'params', NodeType.ARROW],
          ['enter', NodeType.NAMED_PARAMETER, 'value', NodeType.VARIADIC],
          ['enter', NodeType.NAME, 'typeName', NodeType.NAMED_PARAMETER],
          ['leave', NodeType.NAME, 'typeName', NodeType.NAMED_PARAMETER],
          ['leave', NodeType.NAMED_PARAMETER, 'value', NodeType.VARIADIC],
          ['leave', NodeType.VARIADIC, 'params', NodeType.ARROW],
          ['enter', NodeType.NAME, 'returns', NodeType.ARROW],
          ['leave', NodeType.NAME, 'returns', NodeType.ARROW],
          ['leave', NodeType.ARROW, null, null],
        ],
      },
    },
    'NamepathExpr types': {
      'should visit a name node': {
        given: createNameNode('name'),
        then: [
          ['enter', NodeType.NAME, null, null],
          ['leave', NodeType.NAME, null, null],
        ],
      },

      'should visit a member node': {
        given: createMemberNode('child', createNameNode('owner')),
        then: [
          ['enter', NodeType.MEMBER, null, null],
          ['enter', NodeType.NAME, 'owner', NodeType.MEMBER],
          ['leave', NodeType.NAME, 'owner', NodeType.MEMBER],
          ['leave', NodeType.MEMBER, null, null],
        ],
      },

      'should visit a nested member node': {
        given: createMemberNode('superchild', createMemberNode('child', createNameNode('owner'))),
        then: [
          ['enter', NodeType.MEMBER, null, null],
          ['enter', NodeType.MEMBER, 'owner', NodeType.MEMBER],
          ['enter', NodeType.NAME, 'owner', NodeType.MEMBER],
          ['leave', NodeType.NAME, 'owner', NodeType.MEMBER],
          ['leave', NodeType.MEMBER, 'owner', NodeType.MEMBER],
          ['leave', NodeType.MEMBER, null, null],
        ],
      },

      'should visit an inner member node': {
        given: createInnerMemberNode('child', createNameNode('owner')),
        then: [
          ['enter', NodeType.INNER_MEMBER, null, null],
          ['enter', NodeType.NAME, 'owner', NodeType.INNER_MEMBER],
          ['leave', NodeType.NAME, 'owner', NodeType.INNER_MEMBER],
          ['leave', NodeType.INNER_MEMBER, null, null],
        ],
      },

      'should visit a nested inner member node': {
        given: createInnerMemberNode('superchild',
          createInnerMemberNode('child', createNameNode('owner'))),
        then: [
          ['enter', NodeType.INNER_MEMBER, null, null],
          ['enter', NodeType.INNER_MEMBER, 'owner', NodeType.INNER_MEMBER],
          ['enter', NodeType.NAME, 'owner', NodeType.INNER_MEMBER],
          ['leave', NodeType.NAME, 'owner', NodeType.INNER_MEMBER],
          ['leave', NodeType.INNER_MEMBER, 'owner', NodeType.INNER_MEMBER],
          ['leave', NodeType.INNER_MEMBER, null, null],
        ],
      },

      'should visit an instance member node': {
        given: createInstanceMemberNode('child', createNameNode('owner')),
        then: [
          ['enter', NodeType.INSTANCE_MEMBER, null, null],
          ['enter', NodeType.NAME, 'owner', NodeType.INSTANCE_MEMBER],
          ['leave', NodeType.NAME, 'owner', NodeType.INSTANCE_MEMBER],
          ['leave', NodeType.INSTANCE_MEMBER, null, null],
        ],
      },

      'should visit a nested instance member node': {
        given: createInstanceMemberNode('superchild',
          createInstanceMemberNode('child', createNameNode('owner'))),
        then: [
          ['enter', NodeType.INSTANCE_MEMBER, null, null],
          ['enter', NodeType.INSTANCE_MEMBER, 'owner', NodeType.INSTANCE_MEMBER],
          ['enter', NodeType.NAME, 'owner', NodeType.INSTANCE_MEMBER],
          ['leave', NodeType.NAME, 'owner', NodeType.INSTANCE_MEMBER],
          ['leave', NodeType.INSTANCE_MEMBER, 'owner', NodeType.INSTANCE_MEMBER],
          ['leave', NodeType.INSTANCE_MEMBER, null, null],
        ],
      },
    },
    'External': {
      'should visit an external node': {
        given: { type: NodeType.EXTERNAL, name: 'external', quoteStyle: 'double' },
        then: [
          ['enter', NodeType.EXTERNAL, null, null],
          ['leave', NodeType.EXTERNAL, null, null],
        ],
      },
    },
    'Modules': {
      'should visit a module node': {
        given: {
          type: NodeType.MODULE,
          value: createFilePathNode('module'),
        },
        then: [
          ['enter', NodeType.MODULE, null, null],
          ['enter', NodeType.FILE_PATH, 'value', NodeType.MODULE],
          ['leave', NodeType.FILE_PATH, 'value', NodeType.MODULE],
          ['leave', NodeType.MODULE, null, null],
        ],
      },
    },
    'Types with modifiers': {
      'should visit a variadic node': {
        given: { type: NodeType.VARIADIC, value: createNameNode('variadic') },
        then: [
          ['enter', NodeType.VARIADIC, null, null],
          ['enter', NodeType.NAME, 'value', NodeType.VARIADIC],
          ['leave', NodeType.NAME, 'value', NodeType.VARIADIC],
          ['leave', NodeType.VARIADIC, null, null],
        ],
      },
      'should visit an empty variadic node': {
        given: { type: NodeType.VARIADIC, value: null },
        then: [
          // eventName, node && node.type, propName, parentNode && parentNode.type
          ['enter', NodeType.VARIADIC, null, null],
          ['enter', null, 'value', NodeType.VARIADIC],
          ['leave', null, 'value', NodeType.VARIADIC],
          ['leave', NodeType.VARIADIC, null, null],
        ],
      },
      'should visit an optional node': {
        given: {
          type: NodeType.OPTIONAL,
          value: createNameNode('optional'),
        },
        then: [
          ['enter', NodeType.OPTIONAL, null, null],
          ['enter', NodeType.NAME, 'value', NodeType.OPTIONAL],
          ['leave', NodeType.NAME, 'value', NodeType.OPTIONAL],
          ['leave', NodeType.OPTIONAL, null, null],
        ],
      },

      'should visit a nullable node': {
        given: {
          type: NodeType.NULLABLE,
          value: createNameNode('nullable'),
        },
        then: [
          ['enter', NodeType.NULLABLE, null, null],
          ['enter', NodeType.NAME, 'value', NodeType.NULLABLE],
          ['leave', NodeType.NAME, 'value', NodeType.NULLABLE],
          ['leave', NodeType.NULLABLE, null, null],
        ],
      },

      'should visit a non-nullable node': {
        given: {
          type: NodeType.NOT_NULLABLE,
          value: createNameNode('not_nullable'),
        },
        then: [
          ['enter', NodeType.NOT_NULLABLE, null, null],
          ['enter', NodeType.NAME, 'value', NodeType.NOT_NULLABLE],
          ['leave', NodeType.NAME, 'value', NodeType.NOT_NULLABLE],
          ['leave', NodeType.NOT_NULLABLE, null, null],
        ],
      },
    },
    'Type combinations': {
      'should visit a union node': {
        given: createUnionNode(createNameNode('left'), createNameNode('right')),
        then: [
          ['enter', NodeType.UNION, null, null],
          ['enter', NodeType.NAME, 'left', NodeType.UNION],
          ['leave', NodeType.NAME, 'left', NodeType.UNION],
          ['enter', NodeType.NAME, 'right', NodeType.UNION],
          ['leave', NodeType.NAME, 'right', NodeType.UNION],
          ['leave', NodeType.UNION, null, null],
        ],
      },
      'should visit a nested union node': {
        given: createUnionNode(
          createUnionNode(
            createNameNode('left'),
            createNameNode('middle')
          ),
          createNameNode('right')
        ),
        then: [
          ['enter', NodeType.UNION, null, null],
          ['enter', NodeType.UNION, 'left', NodeType.UNION],
          ['enter', NodeType.NAME, 'left', NodeType.UNION],
          ['leave', NodeType.NAME, 'left', NodeType.UNION],
          ['enter', NodeType.NAME, 'right', NodeType.UNION],
          ['leave', NodeType.NAME, 'right', NodeType.UNION],
          ['leave', NodeType.UNION, 'left', NodeType.UNION],
          ['enter', NodeType.NAME, 'right', NodeType.UNION],
          ['leave', NodeType.NAME, 'right', NodeType.UNION],
          ['leave', NodeType.UNION, null, null],
        ],
      },
    },
    'Types with operations': {
      'should visit a type query node': {
        given: createTypeQueryNode(createNameNode('t')),
        then: [
          ['enter', NodeType.TYPE_QUERY, null, null],
          ['enter', NodeType.NAME, 'name', NodeType.TYPE_QUERY],
          ['leave', NodeType.NAME, 'name', NodeType.TYPE_QUERY],
          ['leave', NodeType.TYPE_QUERY, null, null],
        ],
      },

      'should visit a key query node': {
        given: createKeyQueryNode(createNameNode('t')),
        then: [
          ['enter', NodeType.KEY_QUERY, null, null],
          ['enter', NodeType.NAME, 'value', NodeType.KEY_QUERY],
          ['leave', NodeType.NAME, 'value', NodeType.KEY_QUERY],
          ['leave', NodeType.KEY_QUERY, null, null],
        ],
      },

      'should visit an import type node': {
        given: createImportNode(createStringLiteral('jquery')),
        then: [
          ['enter', NodeType.IMPORT, null, null],
          ['enter', NodeType.STRING_VALUE, 'path', NodeType.IMPORT],
          ['leave', NodeType.STRING_VALUE, 'path', NodeType.IMPORT],
          ['leave', NodeType.IMPORT, null, null],
        ],
      },
    },
  };

  entries(testCaseGroups).forEach(function ([testCaseGroup, testCases]) {
    describe(testCaseGroup, function () {
      entries(testCases).forEach(function([testCaseName, testCaseInfo]) {
        it(testCaseName, function() {
          const visitedOrder = [];
          const onEnterSpy = createEventSpy('enter', visitedOrder);
          const onLeaveSpy = createEventSpy('leave', visitedOrder);

          traverse(testCaseInfo.given, onEnterSpy, onLeaveSpy);

          expect(visitedOrder).to.deep.equal(testCaseInfo.then);
        });
      });
    });
  });
});

function createNameNode(name) {
  return {
    type: NodeType.NAME,
    name: name,
  };
}

function createMemberNode(name, owner) {
  return {
    type: NodeType.MEMBER,
    owner: owner,
    name: name,
  };
}

function createUnionNode(left, right) {
  return {
    type: NodeType.UNION,
    left: left,
    right: right,
  };
}

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
  }
}

function createImportNode(path) {
  return {
    type: NodeType.IMPORT,
    path: path,
  }
}

function createStringLiteral(string) {
  return {
    type: NodeType.STRING_VALUE,
    quoteStyle: 'double',
    string: string,
  }
}

function createRecordEntry(key, node) {
  return {
    type: NodeType.RECORD_ENTRY,
    key: key,
    value: node,
  };
}

function createInnerMemberNode(name, owner) {
  return {
    type: NodeType.INNER_MEMBER,
    owner: owner,
    name: name,
  };
}

function createInstanceMemberNode(name, owner) {
  return {
    type: NodeType.INSTANCE_MEMBER,
    owner: owner,
    name: name,
  };
}

function createEventSpy(eventName, result) {
  return function(node, propName, parentNode) {
    result.push([eventName, node && node.type, propName, parentNode && parentNode.type]);
  };
}

function createFilePathNode(filePath) {
  return {
    type: NodeType.FILE_PATH,
    path: filePath,
  };
}
