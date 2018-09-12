"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../../errors");
var typescript_1 = require("../../../typescript");
var callBaseFill_1 = require("../../callBaseFill");
var callBaseGetStructure_1 = require("../../callBaseGetStructure");
function AbstractableNode(Base) {
    return /** @class */ (function (_super) {
        tslib_1.__extends(class_1, _super);
        function class_1() {
            return _super !== null && _super.apply(this, arguments) || this;
        }
        class_1.prototype.isAbstract = function () {
            return this.getAbstractKeyword() != null;
        };
        class_1.prototype.getAbstractKeyword = function () {
            return this.getFirstModifierByKind(typescript_1.SyntaxKind.AbstractKeyword);
        };
        class_1.prototype.getAbstractKeywordOrThrow = function () {
            return errors.throwIfNullOrUndefined(this.getAbstractKeyword(), "Expected to find an abstract keyword.");
        };
        class_1.prototype.setIsAbstract = function (isAbstract) {
            this.toggleModifier("abstract", isAbstract);
            return this;
        };
        class_1.prototype.fill = function (structure) {
            callBaseFill_1.callBaseFill(Base.prototype, this, structure);
            if (structure.isAbstract != null)
                this.setIsAbstract(structure.isAbstract);
            return this;
        };
        class_1.prototype.getStructure = function () {
            return callBaseGetStructure_1.callBaseGetStructure(Base.prototype, this, {
                isAbstract: this.isAbstract()
            });
        };
        return class_1;
    }(Base));
}
exports.AbstractableNode = AbstractableNode;
