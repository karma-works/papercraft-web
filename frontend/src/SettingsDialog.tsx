
import { useState, useEffect } from 'react';
import { Button, Dialog, DialogTrigger, Heading, Modal, Label, Input } from 'react-aria-components';
import { X } from 'lucide-react';
import { SettingsOptions } from './types';

interface SettingsDialogProps {
    options: SettingsOptions | null;
    onSave: (options: SettingsOptions) => void;
    isOpen: boolean;
    onOpenChange: (isOpen: boolean) => void;
}

export default function SettingsDialog({ options, onSave, isOpen, onOpenChange }: SettingsDialogProps) {
    const [formData, setFormData] = useState<SettingsOptions | null>(null);

    useEffect(() => {
        if (options && isOpen) {
            setFormData(JSON.parse(JSON.stringify(options))); // Deep copy
        }
    }, [options, isOpen]);

    if (!formData) return null;

    const handleChange = (key: keyof SettingsOptions, value: any) => {
        setFormData(prev => prev ? ({ ...prev, [key]: value }) : null);
    };

    const handleNestedChange = (parent: keyof SettingsOptions, key: number, value: string) => {
        setFormData(prev => {
            if (!prev) return null;
            const next = { ...prev };
            const parentVal = next[parent];
            if (Array.isArray(parentVal)) {
                const arr = [...parentVal];
                // Special handling for page_size array or margin tuple
                // Assuming direct index mapping for simplified UI
                // But page_size is [w, h].
                // Margin is [t, l, r, b]
                arr[key] = parseFloat(value);
                (next as any)[parent] = arr;
            }
            return next;
        });
    };

    const handleMarginChange = (index: number, value: string) => {
        setFormData(prev => {
            if (!prev) return null;
            const m = [...prev.margin] as [number, number, number, number];
            m[index] = parseFloat(value);
            return { ...prev, margin: m };
        });
    };

    const handleSizeChange = (preset: string) => {
        let size: [number, number] = [210, 297]; // A4 default
        if (preset === 'A4') size = [210, 297];
        if (preset === 'Letter') size = [215.9, 279.4];
        setFormData(prev => prev ? ({ ...prev, page_size: size }) : null);
    };

    const handleSubmit = (e: React.FormEvent) => {
        e.preventDefault();
        if (formData) {
            onSave(formData);
        }
        onOpenChange(false);
    };

    return (
        <DialogTrigger isOpen={isOpen} onOpenChange={onOpenChange}>
            <Button className="hidden">Settings</Button>
            <Modal className="modal-overlay">
                <Dialog className="modal-content settings-dialog">
                    <div className="modal-header">
                        <Heading slot="title">Paper Settings</Heading>
                        <Button onPress={() => onOpenChange(false)} className="close-btn">
                            <X size={20} />
                        </Button>
                    </div>

                    <form onSubmit={handleSubmit} className="settings-form">
                        <div className="form-section">
                            <h3>Page Layout</h3>
                            <div className="form-group">
                                <Label>Preset Size</Label>
                                <div className="btn-group">
                                    <Button type="button" onPress={() => handleSizeChange('A4')} className="btn btn-sm">A4</Button>
                                    <Button type="button" onPress={() => handleSizeChange('Letter')} className="btn btn-sm">Letter</Button>
                                </div>
                            </div>
                            <div className="form-row">
                                <div className="form-group">
                                    <Label>Width (mm)</Label>
                                    <Input
                                        type="number"
                                        value={formData.page_size[0].toString()}
                                        onChange={e => handleNestedChange('page_size', 0, e.target.value)}
                                    />
                                </div>
                                <div className="form-group">
                                    <Label>Height (mm)</Label>
                                    <Input
                                        type="number"
                                        value={formData.page_size[1].toString()}
                                        onChange={e => handleNestedChange('page_size', 1, e.target.value)}
                                    />
                                </div>
                            </div>

                            <div className="form-group">
                                <Label>Margins (Top, Left, Right, Bottom)</Label>
                                <div className="form-row">
                                    <Input type="number" value={formData.margin[0].toString()} onChange={e => handleMarginChange(0, e.target.value)} title="Top" />
                                    <Input type="number" value={formData.margin[1].toString()} onChange={e => handleMarginChange(1, e.target.value)} title="Left" />
                                    <Input type="number" value={formData.margin[2].toString()} onChange={e => handleMarginChange(2, e.target.value)} title="Right" />
                                    <Input type="number" value={formData.margin[3].toString()} onChange={e => handleMarginChange(3, e.target.value)} title="Bottom" />
                                </div>
                            </div>
                        </div>

                        <div className="form-section">
                            <h3>Flaps</h3>
                            <div className="form-row">
                                <div className="form-group">
                                    <Label>Width (mm)</Label>
                                    <Input
                                        type="number"
                                        value={(formData.tab_width || 5).toString()}
                                        onChange={e => handleChange('tab_width', parseFloat(e.target.value))}
                                    />
                                </div>
                                <div className="form-group">
                                    <Label>Angle (deg)</Label>
                                    <Input
                                        type="number"
                                        value={(formData.tab_angle || 45).toString()}
                                        onChange={e => handleChange('tab_angle', parseFloat(e.target.value))}
                                    />
                                </div>
                            </div>
                        </div>

                        <div className="modal-footer">
                            <Button type="button" onPress={() => onOpenChange(false)} className="btn btn-secondary">Cancel</Button>
                            <Button type="submit" className="btn btn-primary">Apply Changes</Button>
                        </div>
                    </form>
                </Dialog>
            </Modal>
        </DialogTrigger>
    );
}
