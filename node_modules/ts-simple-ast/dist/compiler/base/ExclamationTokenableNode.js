"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var manipulation_1 = require("../../manipulation");
var typescript_1 = require("../../typescript");
var utils_1 = require("../../utils");
var callBaseFill_1 = require("../callBaseFill");
function ExclamationTokenableNode(Base) {
    return /** @class */ (function (_super) {
        tslib_1.__extends(class_1, _super);
        function class_1() {
            return _super !== null && _super.apply(this, arguments) || this;
        }
        class_1.prototype.hasExclamationToken = function () {
            return this.compilerNode.exclamationToken != null;
        };
        class_1.prototype.getExclamationTokenNode = function () {
            return this.getNodeFromCompilerNodeIfExists(this.compilerNode.exclamationToken);
        };
        class_1.prototype.getExclamationTokenNodeOrThrow = function () {
            return errors.throwIfNullOrUndefined(this.getExclamationTokenNode(), "Expected to find an exclamation token.");
        };
        class_1.prototype.setHasExclamationToken = function (value) {
            var exclamationTokenNode = this.getExclamationTokenNode();
            var hasExclamationToken = exclamationTokenNode != null;
            if (value === hasExclamationToken)
                return this;
            if (value) {
                if (utils_1.TypeGuards.isQuestionTokenableNode(this))
                    this.setHasQuestionToken(false);
                var colonNode = this.getFirstChildByKind(typescript_1.SyntaxKind.ColonToken);
                if (colonNode == null)
                    throw new errors.InvalidOperationError("Cannot add an exclamation token to a node that does not have a type.");
                manipulation_1.insertIntoParentTextRange({
                    insertPos: colonNode.getStart(),
                    parent: this,
                    newText: "!"
                });
            }
            else
                manipulation_1.removeChildren({ children: [exclamationTokenNode] });
            return this;
        };
        class_1.prototype.fill = function (structure) {
            callBaseFill_1.callBaseFill(Base.prototype, this, structure);
            if (structure.hasExclamationToken != null)
                this.setHasExclamationToken(structure.hasExclamationToken);
            return this;
        };
        return class_1;
    }(Base));
}
exports.ExclamationTokenableNode = ExclamationTokenableNode;
