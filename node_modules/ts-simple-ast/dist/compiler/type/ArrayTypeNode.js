"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var TypeNode_1 = require("./TypeNode");
var ArrayTypeNode = /** @class */ (function (_super) {
    tslib_1.__extends(ArrayTypeNode, _super);
    function ArrayTypeNode() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the array type node's element type node.
     */
    ArrayTypeNode.prototype.getElementTypeNode = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.elementType);
    };
    return ArrayTypeNode;
}(TypeNode_1.TypeNode));
exports.ArrayTypeNode = ArrayTypeNode;
