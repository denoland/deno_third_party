"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../../errors");
var manipulation_1 = require("../../../manipulation");
var typescript_1 = require("../../../typescript");
var utils_1 = require("../../../utils");
var callBaseFill_1 = require("../../callBaseFill");
function InitializerSetExpressionableNode(Base) {
    return /** @class */ (function (_super) {
        tslib_1.__extends(class_1, _super);
        function class_1() {
            return _super !== null && _super.apply(this, arguments) || this;
        }
        class_1.prototype.removeInitializer = function () {
            var initializer = this.getInitializer();
            if (initializer == null)
                return this;
            var previousSibling = initializer.getPreviousSiblingIfKindOrThrow(typescript_1.SyntaxKind.EqualsToken);
            manipulation_1.removeChildren({
                children: [previousSibling, initializer],
                removePrecedingSpaces: true
            });
            return this;
        };
        class_1.prototype.setInitializer = function (textOrWriterFunction) {
            var text = utils_1.getTextFromStringOrWriter(this.getWriterWithQueuedChildIndentation(), textOrWriterFunction);
            errors.throwIfNotStringOrWhitespace(text, "textOrWriterFunction");
            if (this.hasInitializer())
                this.removeInitializer();
            var semiColonToken = this.getLastChildIfKind(typescript_1.SyntaxKind.SemicolonToken);
            manipulation_1.insertIntoParentTextRange({
                insertPos: semiColonToken != null ? semiColonToken.getPos() : this.getEnd(),
                parent: this,
                newText: " = " + text
            });
            return this;
        };
        class_1.prototype.fill = function (structure) {
            callBaseFill_1.callBaseFill(Base.prototype, this, structure);
            if (structure.initializer != null)
                this.setInitializer(structure.initializer);
            return this;
        };
        return class_1;
    }(Base));
}
exports.InitializerSetExpressionableNode = InitializerSetExpressionableNode;
