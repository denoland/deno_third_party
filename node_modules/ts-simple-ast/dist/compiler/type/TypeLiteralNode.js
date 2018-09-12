"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var base_1 = require("../base");
var TypeNode_1 = require("./TypeNode");
exports.TypeLiteralNodeBase = base_1.TypeElementMemberedNode(TypeNode_1.TypeNode);
var TypeLiteralNode = /** @class */ (function (_super) {
    tslib_1.__extends(TypeLiteralNode, _super);
    function TypeLiteralNode() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return TypeLiteralNode;
}(exports.TypeLiteralNodeBase));
exports.TypeLiteralNode = TypeLiteralNode;
