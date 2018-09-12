"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var base_1 = require("../base");
var callBaseFill_1 = require("../callBaseFill");
var statement_1 = require("../statement");
// todo: type node should not be able to return undefined
exports.TypeAliasDeclarationBase = base_1.ChildOrderableNode(base_1.TypeParameteredNode(base_1.TypedNode(base_1.JSDocableNode(base_1.AmbientableNode(base_1.ExportableNode(base_1.ModifierableNode(base_1.NamedNode(statement_1.Statement))))))));
var TypeAliasDeclaration = /** @class */ (function (_super) {
    tslib_1.__extends(TypeAliasDeclaration, _super);
    function TypeAliasDeclaration() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Fills the node from a structure.
     * @param structure - Structure to fill.
     */
    TypeAliasDeclaration.prototype.fill = function (structure) {
        callBaseFill_1.callBaseFill(exports.TypeAliasDeclarationBase.prototype, this, structure);
        return this;
    };
    return TypeAliasDeclaration;
}(exports.TypeAliasDeclarationBase));
exports.TypeAliasDeclaration = TypeAliasDeclaration;
