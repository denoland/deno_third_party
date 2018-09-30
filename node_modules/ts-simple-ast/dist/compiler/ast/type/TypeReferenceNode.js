"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var TypeNode_1 = require("./TypeNode");
var TypeReferenceNode = /** @class */ (function (_super) {
    tslib_1.__extends(TypeReferenceNode, _super);
    function TypeReferenceNode() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the type name.
     */
    TypeReferenceNode.prototype.getTypeName = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.typeName);
    };
    /**
     * Gets the type arguments.
     */
    TypeReferenceNode.prototype.getTypeArguments = function () {
        var _this = this;
        if (this.compilerNode.typeArguments == null)
            return [];
        return this.compilerNode.typeArguments.map(function (a) { return _this.getNodeFromCompilerNode(a); });
    };
    return TypeReferenceNode;
}(TypeNode_1.TypeNode));
exports.TypeReferenceNode = TypeReferenceNode;
