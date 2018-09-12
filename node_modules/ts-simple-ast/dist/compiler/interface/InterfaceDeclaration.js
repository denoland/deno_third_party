"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var utils_1 = require("../../utils");
var base_1 = require("../base");
var callBaseFill_1 = require("../callBaseFill");
var namespace_1 = require("../namespace");
var statement_1 = require("../statement");
exports.InterfaceDeclarationBase = base_1.TypeElementMemberedNode(base_1.ChildOrderableNode(base_1.TextInsertableNode(base_1.ExtendsClauseableNode(base_1.HeritageClauseableNode(base_1.TypeParameteredNode(base_1.JSDocableNode(base_1.AmbientableNode(namespace_1.NamespaceChildableNode(base_1.ExportableNode(base_1.ModifierableNode(base_1.NamedNode(statement_1.Statement))))))))))));
var InterfaceDeclaration = /** @class */ (function (_super) {
    tslib_1.__extends(InterfaceDeclaration, _super);
    function InterfaceDeclaration() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Fills the node from a structure.
     * @param structure - Structure to fill.
     */
    InterfaceDeclaration.prototype.fill = function (structure) {
        callBaseFill_1.callBaseFill(exports.InterfaceDeclarationBase.prototype, this, structure);
        return this;
    };
    /**
     * Gets the base types.
     */
    InterfaceDeclaration.prototype.getBaseTypes = function () {
        return this.getType().getBaseTypes();
    };
    /**
     * Gets the base declarations.
     */
    InterfaceDeclaration.prototype.getBaseDeclarations = function () {
        return utils_1.ArrayUtils.flatten(this.getType().getBaseTypes().map(function (t) {
            var symbol = t.getSymbol();
            return symbol == null ? [] : symbol.getDeclarations();
        }));
    };
    /**
     * Gets all the implementations of the interface.
     *
     * This is similar to "go to implementation."
     */
    InterfaceDeclaration.prototype.getImplementations = function () {
        return this.getNameNode().getImplementations();
    };
    return InterfaceDeclaration;
}(exports.InterfaceDeclarationBase));
exports.InterfaceDeclaration = InterfaceDeclaration;
