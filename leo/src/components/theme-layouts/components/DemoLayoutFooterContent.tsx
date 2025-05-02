import PoweredByLinks from './PoweredByLinks';
import DocumentationButton from './DocumentationButton';
import PurchaseButton from './PurchaseButton';

/**
 * The demo layout footer content.
 */
function DemoLayoutFooterContent() {
	return (
		<>
			<div className="flex grow shrink-0">
				<PurchaseButton className="mx-1" />
				<DocumentationButton className="mx-1" />
			</div>

			<div className="flex grow shrink-0 px-3 justify-end">
				<PoweredByLinks />
			</div>
		</>
	);
}

export default DemoLayoutFooterContent;
