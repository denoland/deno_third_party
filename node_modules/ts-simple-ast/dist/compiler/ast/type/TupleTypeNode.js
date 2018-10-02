"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var TypeNode_1 = require("./TypeNode");
var TupleTypeNode = /** @class */ (function (_super) {
    tslib_1.__extends(TupleTypeNode, _super);
    function TupleTypeNode() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the tuple element type nodes.
     */
    TupleTypeNode.prototype.getElementTypeNodes = function () {
        var _this = this;
        return this.compilerNode.elementTypes.map(function (t) { return _this.getNodeFromCompilerNode(t); });
    };
    return TupleTypeNode;
}(TypeNode_1.TypeNode));
exports.TupleTypeNode = TupleTypeNode;
