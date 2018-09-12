"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var typescript_1 = require("../../typescript");
var utils_1 = require("../../utils");
var callBaseFill_1 = require("../callBaseFill");
function AmbientableNode(Base) {
    return /** @class */ (function (_super) {
        tslib_1.__extends(class_1, _super);
        function class_1() {
            return _super !== null && _super.apply(this, arguments) || this;
        }
        class_1.prototype.hasDeclareKeyword = function () {
            return this.getDeclareKeyword() != null;
        };
        class_1.prototype.getDeclareKeywordOrThrow = function () {
            return errors.throwIfNullOrUndefined(this.getDeclareKeyword(), "Expected to find a declare keyword.");
        };
        class_1.prototype.getDeclareKeyword = function () {
            return this.getFirstModifierByKind(typescript_1.SyntaxKind.DeclareKeyword);
        };
        class_1.prototype.isAmbient = function () {
            return utils_1.isNodeAmbientOrInAmbientContext(this);
        };
        class_1.prototype.setHasDeclareKeyword = function (value) {
            // do nothing for these kind of nodes
            if (utils_1.TypeGuards.isInterfaceDeclaration(this) || utils_1.TypeGuards.isTypeAliasDeclaration(this))
                return this;
            this.toggleModifier("declare", value);
            return this;
        };
        class_1.prototype.fill = function (structure) {
            callBaseFill_1.callBaseFill(Base.prototype, this, structure);
            if (structure.hasDeclareKeyword != null)
                this.setHasDeclareKeyword(structure.hasDeclareKeyword);
            return this;
        };
        return class_1;
    }(Base));
}
exports.AmbientableNode = AmbientableNode;
