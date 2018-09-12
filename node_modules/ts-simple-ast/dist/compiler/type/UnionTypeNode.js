"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var TypeNode_1 = require("./TypeNode");
var UnionTypeNode = /** @class */ (function (_super) {
    tslib_1.__extends(UnionTypeNode, _super);
    function UnionTypeNode() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the union type nodes.
     */
    UnionTypeNode.prototype.getTypeNodes = function () {
        var _this = this;
        return this.compilerNode.types.map(function (t) { return _this.getNodeFromCompilerNode(t); });
    };
    return UnionTypeNode;
}(TypeNode_1.TypeNode));
exports.UnionTypeNode = UnionTypeNode;
