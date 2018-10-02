"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var TypeNode_1 = require("./TypeNode");
var IntersectionTypeNode = /** @class */ (function (_super) {
    tslib_1.__extends(IntersectionTypeNode, _super);
    function IntersectionTypeNode() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the intersection type nodes.
     */
    IntersectionTypeNode.prototype.getTypeNodes = function () {
        var _this = this;
        return this.compilerNode.types.map(function (t) { return _this.getNodeFromCompilerNode(t); });
    };
    return IntersectionTypeNode;
}(TypeNode_1.TypeNode));
exports.IntersectionTypeNode = IntersectionTypeNode;
