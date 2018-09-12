"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var manipulation_1 = require("../../manipulation");
var utils_1 = require("../../utils");
var base_1 = require("../base");
var callBaseFill_1 = require("../callBaseFill");
var TypeElement_1 = require("./TypeElement");
exports.IndexSignatureDeclarationBase = base_1.ChildOrderableNode(base_1.JSDocableNode(base_1.ReadonlyableNode(base_1.ModifierableNode(TypeElement_1.TypeElement))));
var IndexSignatureDeclaration = /** @class */ (function (_super) {
    tslib_1.__extends(IndexSignatureDeclaration, _super);
    function IndexSignatureDeclaration() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Fills the node from a structure.
     * @param structure - Structure to fill.
     */
    IndexSignatureDeclaration.prototype.fill = function (structure) {
        callBaseFill_1.callBaseFill(exports.IndexSignatureDeclarationBase.prototype, this, structure);
        if (structure.keyName != null)
            this.setKeyName(structure.keyName);
        if (structure.keyType != null)
            this.setKeyType(structure.keyType);
        if (structure.returnType != null)
            this.setReturnType(structure.returnType);
        return this;
    };
    /**
     * Gets the key name.
     */
    IndexSignatureDeclaration.prototype.getKeyName = function () {
        return this.getKeyNameNode().getText();
    };
    /**
     * Sets the key name.
     * @param name - New name.
     */
    IndexSignatureDeclaration.prototype.setKeyName = function (name) {
        if (this.getKeyName() === name)
            return;
        this.getKeyNameNode().replaceWithText(name, this.getWriterWithQueuedChildIndentation());
    };
    /**
     * Gets the key name node.
     */
    IndexSignatureDeclaration.prototype.getKeyNameNode = function () {
        var param = this.compilerNode.parameters[0];
        return this.getNodeFromCompilerNode(param.name);
    };
    /**
     * Gets the key type.
     */
    IndexSignatureDeclaration.prototype.getKeyType = function () {
        return this.getKeyTypeNode().getType();
    };
    /**
     * Sets the key type.
     * @param type - Type.
     */
    IndexSignatureDeclaration.prototype.setKeyType = function (type) {
        if (this.getKeyTypeNode().getText() === type)
            return;
        this.getKeyTypeNode().replaceWithText(type, this.getWriterWithQueuedChildIndentation());
    };
    /**
     * Gets the key type node.
     */
    IndexSignatureDeclaration.prototype.getKeyTypeNode = function () {
        var param = this.compilerNode.parameters[0];
        return this.getNodeFromCompilerNode(param.type);
    };
    /**
     * Gets the return type.
     */
    IndexSignatureDeclaration.prototype.getReturnType = function () {
        return this.getReturnTypeNode().getType();
    };
    /**
     * Gets the return type node.
     */
    IndexSignatureDeclaration.prototype.getReturnTypeNode = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.type);
    };
    IndexSignatureDeclaration.prototype.setReturnType = function (textOrWriterFunction) {
        var returnTypeNode = this.getReturnTypeNode();
        var text = utils_1.getTextFromStringOrWriter(this.getWriterWithQueuedChildIndentation(), textOrWriterFunction);
        if (returnTypeNode.getText() === text)
            return this;
        returnTypeNode.replaceWithText(text);
        return this;
    };
    /**
     * Removes this index signature.
     */
    IndexSignatureDeclaration.prototype.remove = function () {
        manipulation_1.removeInterfaceMember(this);
    };
    return IndexSignatureDeclaration;
}(exports.IndexSignatureDeclarationBase));
exports.IndexSignatureDeclaration = IndexSignatureDeclaration;
